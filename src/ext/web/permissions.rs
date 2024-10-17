use deno_core::anyhow::anyhow;
use std::{
    borrow::Cow,
    cell::RefCell,
    collections::HashSet,
    path::{Path, PathBuf},
    rc::Rc,
};

/// The default permissions manager for the web related extensions
/// Allows all operations
pub struct DefaultWebPermissions;
impl WebPermissions for DefaultWebPermissions {}

// Inner container for the allowlist permission set
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

    fn check_read<'a>(
        &self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_core::error::AnyError> {
        if self.0.borrow().read_paths.contains(p.to_str().unwrap()) {
            Ok(Cow::Borrowed(p))
        } else {
            Err(anyhow!("Path '{}' is not allowed to be read", p.display()))
        }
    }

    fn check_write<'a>(
        &self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_core::error::AnyError> {
        if self.0.borrow().write_paths.contains(p.to_str().unwrap()) {
            Ok(Cow::Borrowed(p))
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
    fn check_read<'a>(
        &self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_core::error::AnyError> {
        Ok(Cow::Borrowed(p))
    }

    /// Check if a path is allowed to be written to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write<'a>(
        &self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_core::error::AnyError> {
        Ok(Cow::Borrowed(p))
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

    fn check_read<'a>(
        &mut self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_core::error::AnyError> {
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

    fn check_read<'a>(
        &mut self,
        p: &'a str,
        api_name: &str,
    ) -> Result<PathBuf, deno_core::error::AnyError> {
        self.0
            .check_read(Path::new(p), api_name)
            .map(|p| p.into_owned())
    }

    fn check_write(
        &mut self,
        p: &str,
        api_name: &str,
    ) -> Result<PathBuf, deno_core::error::AnyError> {
        self.0
            .check_write(Path::new(p), api_name)
            .map(|p| p.into_owned())
    }

    fn check_write_path<'a>(
        &mut self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, deno_core::error::AnyError> {
        self.0.check_write(p, api_name)
    }
}
