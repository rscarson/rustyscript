use deno_core::{anyhow::anyhow, extension, Extension};
use std::{cell::RefCell, collections::HashSet, rc::Rc, sync::Arc};

/// The default permissions manager for the web related extensions
/// Allows all operations
pub struct DefaultWebPermissions;
impl WebPermissions for DefaultWebPermissions {}

#[derive(Clone, Default)]
struct AllowlistWebPermissionsSet {
    pub hrtime: bool,
    pub url: HashSet<String>,
    pub read_paths: HashSet<String>,
    pub write_paths: HashSet<String>,
    pub hosts: HashSet<String>,
}

/// Permissions manager for the web related extensions
/// Allows only operations that are explicitly enabled
/// Uses interior mutability to allow changing the permissions at runtime
#[derive(Clone, Default)]
pub struct AllowlistWebPermissions(Rc<RefCell<AllowlistWebPermissionsSet>>);
impl AllowlistWebPermissions {
    /// Create a new instance with nothing allowed by default
    #[must_use]
    pub fn new() -> Self {
        Self(Rc::new(RefCell::new(AllowlistWebPermissionsSet::default())))
    }

    /// Set the `hrtime` permission
    /// If true, timers will be allowed to use high resolution time
    pub fn set_hrtime(&self, value: bool) {
        self.0.borrow_mut().hrtime = value;
    }

    /// Whitelist a URL
    pub fn allow_url(&self, url: &str) {
        self.0.borrow_mut().url.insert(url.to_string());
    }

    /// Blacklist a URL
    pub fn deny_url(&self, url: &str) {
        self.0.borrow_mut().url.remove(url);
    }

    /// Whitelist a path for reading
    pub fn allow_read(&self, path: &str) {
        self.0.borrow_mut().read_paths.insert(path.to_string());
    }

    /// Blacklist a path for reading
    pub fn deny_read(&self, path: &str) {
        self.0.borrow_mut().read_paths.remove(path);
    }

    /// Whitelist a path for writing
    pub fn allow_write(&self, path: &str) {
        self.0.borrow_mut().write_paths.insert(path.to_string());
    }

    /// Blacklist a path for writing
    pub fn deny_write(&self, path: &str) {
        self.0.borrow_mut().write_paths.remove(path);
    }

    /// Whitelist a host
    pub fn allow_host(&self, host: &str) {
        self.0.borrow_mut().hosts.insert(host.to_string());
    }

    /// Blacklist a host
    pub fn deny_host(&self, host: &str) {
        self.0.borrow_mut().hosts.remove(host);
    }
}
impl WebPermissions for AllowlistWebPermissions {
    fn allow_hrtime(&self) -> bool {
        self.0.borrow().hrtime
    }

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        if self.0.borrow().hosts.contains(host) {
            Ok(())
        } else {
            Err(anyhow!("Host '{}' is not allowed", host))
        }
    }

    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        if self.0.borrow().url.contains(url.as_str()) {
            Ok(())
        } else {
            Err(anyhow!("URL '{}' is not allowed", url))
        }
    }

    fn check_read(
        &self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        if self.0.borrow().read_paths.contains(p.to_str().unwrap()) {
            Ok(())
        } else {
            Err(anyhow!("Path '{}' is not allowed to be read", p.display()))
        }
    }

    fn check_write(
        &self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        if self.0.borrow().write_paths.contains(p.to_str().unwrap()) {
            Ok(())
        } else {
            Err(anyhow!(
                "Path '{}' is not allowed to be written to",
                p.display()
            ))
        }
    }
}

/// Trait managing the permissions for the web related extensions
/// See [`DefaultWebPermissions`] for a default implementation that allows-all
pub trait WebPermissions {
    /// Check if `hrtime` is allowed
    /// If true, timers will be allowed to use high resolution time
    fn allow_hrtime(&self) -> bool {
        true
    }

    /// Check if a URL is allowed to be used by fetch or websocket
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    /// Check if a path is allowed to be read by fetch or net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read(
        &self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    /// Check if a path is allowed to be written to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write(
        &self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }

    /// Check if a host is allowed to be connected to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        Ok(())
    }
}

#[derive(Clone)]
pub struct PermissionsContainer(pub Rc<dyn WebPermissions>);
impl deno_web::TimersPermission for PermissionsContainer {
    fn allow_hrtime(&mut self) -> bool {
        self.0.allow_hrtime()
    }
}
impl deno_fetch::FetchPermissions for PermissionsContainer {
    fn check_net_url(
        &mut self,
        url: &reqwest::Url,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        self.0.check_url(url, api_name)
    }

    fn check_read(
        &mut self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        self.0.check_read(p, api_name)
    }
}
impl deno_net::NetPermissions for PermissionsContainer {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        self.0.check_host(host.0.as_ref(), host.1, api_name)
    }

    fn check_read(
        &mut self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        self.0.check_read(p, api_name)
    }

    fn check_write(
        &mut self,
        p: &std::path::Path,
        api_name: &str,
    ) -> Result<(), deno_core::error::AnyError> {
        self.0.check_write(p, api_name)
    }
}

/// Options for configuring the web related extensions
#[derive(Clone)]
pub struct WebOptions {
    /// Base URL for some `deno_web` OPs
    pub base_url: Option<deno_core::ModuleSpecifier>,

    /// User agent to use for fetch
    pub user_agent: String,

    /// Root certificate store for TLS connections for fetches and network OPs
    pub root_cert_store_provider: Option<std::sync::Arc<dyn deno_tls::RootCertStoreProvider>>,

    /// Proxy for fetch
    pub proxy: Option<deno_tls::Proxy>,

    /// Request builder hook for fetch
    pub request_builder_hook: Option<
        fn(
            _: deno_fetch::reqwest::RequestBuilder,
        ) -> Result<deno_fetch::reqwest::RequestBuilder, deno_core::error::AnyError>,
    >,

    /// If true, fetches and network OPs will ignore SSL errors
    pub unsafely_ignore_certificate_errors: Option<Vec<String>>,

    /// Client certificate and key for fetch
    pub client_cert_chain_and_key: deno_tls::TlsKeys,

    /// File fetch handler for fetch
    pub file_fetch_handler: std::rc::Rc<dyn deno_fetch::FetchHandler>,

    /// Permissions manager for the web related extensions
    pub permissions: Rc<dyn WebPermissions>,
}

impl Default for WebOptions {
    fn default() -> Self {
        Self {
            base_url: None,
            user_agent: String::new(),
            root_cert_store_provider: None,
            proxy: None,
            request_builder_hook: None,
            unsafely_ignore_certificate_errors: None,
            client_cert_chain_and_key: deno_tls::TlsKeys::Null,
            file_fetch_handler: std::rc::Rc::new(deno_fetch::DefaultFileFetchHandler),
            permissions: Rc::new(DefaultWebPermissions),
        }
    }
}

extension!(
    init_web,
    deps = [rustyscript],
    esm_entry_point = "ext:init_web/init_web.js",
    esm = [ dir "src/ext/web", "init_web.js" ],
    options = {
        permissions: Rc<dyn WebPermissions>
    },
    state = |state, config| state.put(PermissionsContainer(config.permissions)),
);

extension!(
    init_fetch,
    deps = [rustyscript],
    esm_entry_point = "ext:init_fetch/init_fetch.js",
    esm = [ dir "src/ext/web", "init_fetch.js" ],
);

extension!(
    init_net,
    deps = [rustyscript],
    esm_entry_point = "ext:init_net/init_net.js",
    esm = [ dir "src/ext/web", "init_net.js" ],
);

pub fn extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops_and_esm::<PermissionsContainer>(
            Arc::default(),
            options.base_url.clone(),
        ),
        deno_net::deno_net::init_ops_and_esm::<PermissionsContainer>(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        ),
        deno_fetch::deno_fetch::init_ops_and_esm::<PermissionsContainer>(deno_fetch::Options {
            user_agent: options.user_agent,
            root_cert_store_provider: options.root_cert_store_provider,
            proxy: options.proxy,
            request_builder_hook: options.request_builder_hook,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors,
            client_cert_chain_and_key: options.client_cert_chain_and_key,
            file_fetch_handler: options.file_fetch_handler,
        }),
        init_web::init_ops_and_esm(options.permissions),
        init_fetch::init_ops_and_esm(),
        init_net::init_ops_and_esm(),
    ]
}

pub fn snapshot_extensions(options: WebOptions) -> Vec<Extension> {
    vec![
        deno_web::deno_web::init_ops::<PermissionsContainer>(
            Arc::default(),
            options.base_url.clone(),
        ),
        deno_net::deno_net::init_ops::<PermissionsContainer>(
            options.root_cert_store_provider.clone(),
            options.unsafely_ignore_certificate_errors.clone(),
        ),
        deno_fetch::deno_fetch::init_ops::<PermissionsContainer>(deno_fetch::Options {
            user_agent: options.user_agent,
            root_cert_store_provider: options.root_cert_store_provider,
            proxy: options.proxy,
            request_builder_hook: options.request_builder_hook,
            unsafely_ignore_certificate_errors: options.unsafely_ignore_certificate_errors,
            client_cert_chain_and_key: options.client_cert_chain_and_key,
            file_fetch_handler: options.file_fetch_handler,
        }),
        init_web::init_ops(options.permissions),
        init_fetch::init_ops(),
        init_net::init_ops(),
    ]
}
