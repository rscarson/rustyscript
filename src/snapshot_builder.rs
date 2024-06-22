use crate::{
    ext,
    inner_runtime::{InnerRuntime, InnerRuntimeOptions},
    module_loader::RustyLoader,
    traits::ToModuleSpecifier,
    transpiler::{self, transpile_extension},
    Error, Module,
};
use deno_core::{JsRuntimeForSnapshot, ModuleId, PollEventLoopOptions, RuntimeOptions};
use std::rc::Rc;

/// A more restricted version of the `Runtime` struct that is used to create a snapshot of the runtime state
/// This runtime should ONLY be used to create a snapshot, and not for normal use
///
/// Snapshots can be used to massively decrease the startup time of a Runtime instance (15ms -> 3ms) by pre-loading
/// extensions and modules into the runtime state before it is created. A snapshot can be used on any runtime with
/// the same set of extensions and options as the runtime that created it.
///
/// This struct is only available when the `snapshot_builder` feature is enabled
/// Once you've set up the runtime, you can call `into_snapshot` to get the snapshot
///
/// You should save it to a file and load it with `include_bytes!` in order to use it
/// in the `RuntimeOptions` struct's `startup_snapshot` field
///
/// # Example
///
/// ```rust
/// use rustyscript::{SnapshotBuilder, Module, Error};
/// use std::fs;
///
/// # fn main() -> Result<(), Error> {
/// let module = Module::new("example.js", "export function example() { return 42; }");
/// let snapshot = SnapshotBuilder::new(Default::default())?
///    .with_module(&module)?
///    .finish();
///
/// // Save the snapshot to a file
/// fs::write("snapshot.bin", snapshot)?;
///
/// // To use the snapshot, load it with `include_bytes!` into the `RuntimeOptions` struct:
/// // const STARTUP_SNAPSHOT: &[u8] = include_bytes!("snapshot.bin");
/// // RuntimeOptions {
/// //     startup_snapshot: Some(STARTUP_SNAPSHOT),
/// //     ..Default::default()
/// // };
///
/// # Ok(())
/// # }
/// ```
pub struct SnapshotBuilder {
    deno_runtime: JsRuntimeForSnapshot,
    options: InnerRuntimeOptions,
}
impl SnapshotBuilder {
    /// Creates a new snapshot builder with the given options
    pub fn new(options: InnerRuntimeOptions) -> Result<Self, Error> {
        let loader = Rc::new(RustyLoader::new(options.module_cache));

        // If a snapshot is provided, do not reload ops
        let extensions = if options.startup_snapshot.is_some() {
            ext::all_snapshot_extensions(options.extensions, options.extension_options)
        } else {
            ext::all_extensions(options.extensions, options.extension_options)
        };

        Ok(Self {
            deno_runtime: JsRuntimeForSnapshot::try_new(RuntimeOptions {
                module_loader: Some(loader.clone()),

                extension_transpiler: Some(Rc::new(|specifier, code| {
                    transpile_extension(specifier, code)
                })),

                source_map_getter: Some(loader),

                startup_snapshot: options.startup_snapshot,
                extensions,

                ..Default::default()
            })?,

            options: InnerRuntimeOptions {
                timeout: options.timeout,
                default_entrypoint: options.default_entrypoint,
                ..Default::default()
            },
        })
    }

    /// Executes the given module, on the runtime, making it available to be
    /// imported by other modules in this runtime, and those that will use the
    /// snapshot
    pub fn with_module(mut self, module: &Module) -> Result<Self, Error> {
        self.load_module(module)?;
        Ok(self)
    }

    /// Executes a piece of non-ECMAScript-module JavaScript code on the runtime
    /// This code can be used to set up the runtime state before creating the snapshot
    pub fn with_expression(mut self, expr: &str) -> Result<Self, Error> {
        self.deno_runtime.execute_script("", expr.to_string())?;
        Ok(self)
    }

    /// Consumes the runtime and returns a snapshot of the runtime state
    /// This is only available when the `snapshot_builder` feature is enabled
    /// and will return a `Box<[u8]>` representing the snapshot
    ///
    /// To use the snapshot, provide it, as a static slice, in [`RuntimeOptions::startup_snapshot`]
    /// Therefore, in order to use this snapshot, make sure you write it to a file and load it with
    /// `include_bytes!`
    ///
    /// WARNING: In order to use the snapshot, make sure the runtime using it is
    /// provided the same extensions and options as the original runtime. Any extensions
    /// you provided must be loaded with `init_ops` instead of `init_ops_and_esm`.
    pub fn finish(self) -> Box<[u8]> {
        let deno_rt: JsRuntimeForSnapshot = self.deno_runtime;
        deno_rt.snapshot()
    }

    /// Loads a module into the runtime, making it available to be
    /// imported by other modules in this runtime, and those that will use the
    /// snapshot
    ///
    /// WARNING: Returned module id is not guaranteed to be the same when the snapshot is loaded
    /// Possibly resulting in a runtime panic
    pub fn load_module(&mut self, module: &Module) -> Result<ModuleId, Error> {
        let timeout = self.options.timeout;
        let deno_runtime = &mut self.deno_runtime;

        InnerRuntime::run_async_task(
            async move {
                let module_specifier = module.filename().to_module_specifier()?;
                let (code, _) = transpiler::transpile(&module_specifier, module.contents())?;
                let code = deno_core::FastString::from(code);

                let modid = deno_runtime
                    .load_side_es_module_from_code(&module_specifier, code)
                    .await?;
                let result = deno_runtime.mod_evaluate(modid);
                deno_runtime
                    .run_event_loop(PollEventLoopOptions::default())
                    .await?;
                result.await?;
                Ok::<ModuleId, Error>(modid)
            },
            timeout,
        )
    }
}
