use super::node::RustyResolver;
use super::web::PermissionsContainer;
use super::{ExtensionOptions, ExtensionTrait};
use crate::module_loader::{LoaderOptions, RustyLoader};
use ::deno_permissions::Permissions;
use deno_core::v8::{BackingStore, SharedRef};
use deno_core::{extension, CrossIsolateStore, Extension, FeatureChecker};
use deno_fs::RealFs;
use deno_runtime::permissions::RuntimePermissionDescriptorParser;
use std::collections::HashSet;
use std::rc::Rc;
use std::sync::Arc;

fn build_permissions(
    permissions_container: &PermissionsContainer,
) -> ::deno_permissions::PermissionsContainer {
    let fs = Arc::new(RealFs);
    let permission_desc_parser = Arc::new(RuntimePermissionDescriptorParser::new(fs.clone()));
    ::deno_permissions::PermissionsContainer::new(permission_desc_parser, Permissions::allow_all())
}

// Some of the polyfills reference the denoland/deno runtime directly
// So we need to include a subset of the real thing
//
// However that extension lists nearly all others as dependencies so
// It will always be the last initialized extension
extension!(
    init_runtime,
    esm_entry_point = "ext:init_runtime/init_runtime.js",
    esm = [ dir "src/ext/runtime", "init_runtime.js" ],
    state = |state| {
        let options = BootstrapOptions {
            no_color: false,
            args: vec![
                "--colors".to_string(),
            ],
            ..BootstrapOptions::default()
        };
        state.put(options);

        let container = state.borrow::<PermissionsContainer>();
        let permissions = build_permissions(container);
        state.put(permissions);
    }
);
impl ExtensionTrait<()> for init_runtime {
    fn init((): ()) -> Extension {
        init_runtime::init()
    }
}

impl ExtensionTrait<()> for deno_runtime::runtime {
    fn init((): ()) -> Extension {
        let mut e = deno_runtime::runtime::init();
        e.esm_entry_point = None;
        e
    }
}

use deno_runtime::fmt_errors::format_js_error;
use deno_runtime::ops::permissions::deno_permissions;
impl ExtensionTrait<()> for deno_permissions {
    fn init((): ()) -> Extension {
        deno_permissions::init()
    }
}

use deno_runtime::ops::worker_host::{deno_worker_host, CreateWebWorkerCb};
impl
    ExtensionTrait<(
        &ExtensionOptions,
        Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    )> for deno_worker_host
{
    fn init(
        options: (
            &ExtensionOptions,
            Option<CrossIsolateStore<SharedRef<BackingStore>>>,
        ),
    ) -> Extension {
        let options = WebWorkerCallbackOptions::new(options.0, options.1);
        let callback = create_web_worker_callback(options);
        deno_worker_host::init(callback, None)
    }
}

use deno_runtime::ops::web_worker::deno_web_worker;
impl ExtensionTrait<()> for deno_web_worker {
    fn init((): ()) -> Extension {
        deno_web_worker::init()
    }
}

use deno_runtime::ops::process::deno_process;
impl ExtensionTrait<Arc<RustyResolver>> for deno_process {
    fn init(resolver: Arc<RustyResolver>) -> Extension {
        deno_process::init(Some(resolver))
    }
}

use deno_runtime::ops::signal::deno_signal;
impl ExtensionTrait<()> for deno_signal {
    fn init((): ()) -> Extension {
        deno_signal::init()
    }
}

use deno_runtime::ops::os::deno_os;
impl ExtensionTrait<()> for deno_os {
    fn init((): ()) -> Extension {
        deno_os::init(ExitCode::default())
    }
}

use deno_runtime::ops::bootstrap::deno_bootstrap;
impl ExtensionTrait<()> for deno_bootstrap {
    fn init((): ()) -> Extension {
        deno_bootstrap::init(None)
    }
}

use deno_runtime::ops::fs_events::deno_fs_events;
impl ExtensionTrait<()> for deno_fs_events {
    fn init((): ()) -> Extension {
        deno_fs_events::init()
    }
}

pub fn extensions(
    options: &ExtensionOptions,
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    is_snapshot: bool,
) -> Vec<Extension> {
    vec![
        deno_fs_events::build((), is_snapshot),
        deno_bootstrap::build((), is_snapshot),
        deno_os::build((), is_snapshot),
        deno_signal::build((), is_snapshot),
        deno_process::build(options.node_resolver.clone(), is_snapshot),
        deno_web_worker::build((), is_snapshot),
        deno_worker_host::build((options, shared_array_buffer_store), is_snapshot),
        deno_permissions::build((), is_snapshot),
        //
        deno_runtime::runtime::build((), is_snapshot),
        init_runtime::build((), is_snapshot),
    ]
}

use deno_runtime::web_worker::{WebWorker, WebWorkerOptions, WebWorkerServiceOptions};
use deno_runtime::worker::ExitCode;
use deno_runtime::{colors, BootstrapOptions, WorkerExecutionMode, WorkerLogLevel};
#[derive(Clone)]
pub struct WebWorkerCallbackOptions {
    shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    node_resolver: Arc<RustyResolver>,
    root_cert_store_provider: Option<Arc<dyn deno_tls::RootCertStoreProvider>>,
    broadcast_channel: deno_broadcast_channel::InMemoryBroadcastChannel,
    unsafely_ignore_certificate_errors: Option<Vec<String>>,
    seed: Option<u64>,
    stdio: deno_io::Stdio,
    blob_store: Arc<deno_web::BlobStore>,
}
impl WebWorkerCallbackOptions {
    pub fn new(
        options: &ExtensionOptions,
        shared_array_buffer_store: Option<CrossIsolateStore<SharedRef<BackingStore>>>,
    ) -> Self {
        Self {
            shared_array_buffer_store,
            node_resolver: options.node_resolver.clone(),
            root_cert_store_provider: options.web.root_cert_store_provider.clone(),
            broadcast_channel: options.broadcast_channel.clone(),
            unsafely_ignore_certificate_errors: options
                .web
                .unsafely_ignore_certificate_errors
                .clone(),
            seed: options.crypto_seed,
            stdio: options.io_pipes.clone().unwrap_or_default(),
            blob_store: options.web.blob_store.clone(),
        }
    }
}

// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
fn create_web_worker_callback(options: WebWorkerCallbackOptions) -> Arc<CreateWebWorkerCb> {
    Arc::new(move |args| {
        let node_resolver = options.node_resolver.clone();
        let module_loader = Rc::new(RustyLoader::new(LoaderOptions {
            cache_provider: None,
            import_provider: None,
            schema_whlist: HashSet::default(),
            node_resolver: node_resolver.clone(),
            ..Default::default()
        }));

        let create_web_worker_cb = create_web_worker_callback(options.clone());

        let mut feature_checker = FeatureChecker::default();
        feature_checker.set_exit_cb(Box::new(|_, _| {}));

        let services = WebWorkerServiceOptions {
            root_cert_store_provider: options.root_cert_store_provider.clone(),
            module_loader,
            fs: node_resolver.filesystem(),
            node_services: Some(node_resolver.init_services()),
            blob_store: options.blob_store.clone(),
            broadcast_channel: options.broadcast_channel.clone(),
            shared_array_buffer_store: options.shared_array_buffer_store.clone(),
            compiled_wasm_module_store: None,
            maybe_inspector_server: None,
            feature_checker: feature_checker.into(),
            npm_process_state_provider: Some(node_resolver.clone()),
            permissions: args.permissions,
        };
        let options = WebWorkerOptions {
            name: args.name,
            main_module: args.main_module.clone(),
            worker_id: args.worker_id,
            bootstrap: BootstrapOptions {
                deno_version: env!("CARGO_PKG_VERSION").to_string(),
                args: vec![],
                cpu_count: std::thread::available_parallelism()
                    .map(std::num::NonZero::get)
                    .unwrap_or(1),
                log_level: WorkerLogLevel::default(),
                enable_op_summary_metrics: false,
                enable_testing_features: false,
                locale: deno_core::v8::icu::get_language_tag(),
                location: Some(args.main_module),
                no_color: !colors::use_color(),
                color_level: colors::get_color_level(),
                is_stdout_tty: false,
                is_stderr_tty: false,
                unstable_features: vec![],
                user_agent: concat!("rustyscript_", env!("CARGO_PKG_VERSION")).to_string(),
                inspect: false,
                has_node_modules_dir: node_resolver.has_node_modules_dir(),
                argv0: None,
                node_debug: None,
                node_ipc_fd: None,
                mode: WorkerExecutionMode::Worker,
                serve_port: None,
                serve_host: None,
                otel_config: None,
            },
            extensions: vec![],
            startup_snapshot: None,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors.clone(),
            seed: options.seed,
            create_web_worker_cb,
            format_js_error_fn: Some(Arc::new(format_js_error)),
            worker_type: args.worker_type,
            get_error_class_fn: Some(&get_error_class_name),
            stdio: options.stdio.clone(),
            cache_storage_dir: None,
            strace_ops: None,
            close_on_idle: false,
            maybe_worker_metadata: None,
            create_params: None,
            enable_stack_trace_arg_in_ops: false,
        };
        WebWorker::bootstrap_from_options(services, options)
    })
}

pub fn get_error_class_name(e: &deno_core::error::AnyError) -> &'static str {
    deno_runtime::errors::get_error_class_name(e)
        .or_else(|| {
            e.downcast_ref::<deno_ast::ParseDiagnostic>()
                .map(|_| "SyntaxError")
        })
        .or_else(|| {
            e.downcast_ref::<std::num::TryFromIntError>()
                .map(|_| "TypeError")
        })
        .unwrap_or("Error")
}
