use deno_permissions::{PathResolveError, PermissionCheckError};
use std::{
    borrow::Cow,
    collections::HashSet,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};

/// Wrapper error for deno permissions checks
/// This will resolve to `PermissionCheckError::PermissionDeniedError`
/// This type is needed since `deno_permissions` does not expose any way to
/// externally create a `PermissionCheckError`
pub struct PermissionDenied {
    pub access: String,
    pub name: &'static str,
}
impl PermissionDenied {
    pub fn new(access: impl ToString, reason: &'static str) -> Self {
        Self {
            access: access.to_string(),
            name: reason,
        }
    }

    pub fn oops<T>(access: impl ToString) -> Result<T, Self> {
        Err(Self::new(access, "Not Allowed"))
    }
}

// Nonsense error for now
impl From<PermissionDenied> for PermissionCheckError {
    fn from(e: PermissionDenied) -> Self {
        PermissionCheckError::PathResolve(PathResolveError::EmptyPath)
    }
}

/// The default permissions manager for the web related extensions
/// Allows all operations
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultWebPermissions;
impl WebPermissions for DefaultWebPermissions {
    fn allow_hrtime(&self) -> bool {
        true
    }

    fn check_url(&self, url: &deno_core::url::Url, api_name: &str) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: &'a Path,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>> {
        Some(Cow::Borrowed(path))
    }

    fn check_read<'a>(
        &self,
        p: &'a Path,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDenied> {
        Ok(Cow::Borrowed(p))
    }

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_write<'a>(
        &self,
        p: &'a Path,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDenied> {
        Ok(Cow::Borrowed(p))
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_write_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_write_partial(
        &self,
        path: &str,
        api_name: &str,
    ) -> Result<std::path::PathBuf, PermissionDenied> {
        Ok(PathBuf::from(path))
    }

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_env(&self, var: &str) -> Result<(), PermissionDenied> {
        Ok(())
    }

    fn check_exec(&self) -> Result<(), PermissionDenied> {
        Ok(())
    }
}

// Inner container for the allowlist permission set
#[derive(Clone, Default, Debug)]
#[allow(clippy::struct_excessive_bools)]
struct AllowlistWebPermissionsSet {
    pub hrtime: bool,
    pub exec: bool,
    pub read_all: bool,
    pub write_all: bool,
    pub url: HashSet<String>,
    pub openr_paths: HashSet<String>,
    pub openw_paths: HashSet<String>,
    pub envs: HashSet<String>,
    pub sys: HashSet<SystemsPermissionKind>,
    pub read_paths: HashSet<String>,
    pub write_paths: HashSet<String>,
    pub hosts: HashSet<String>,
}

/// Permissions manager for the web related extensions
/// Allows only operations that are explicitly enabled
/// Uses interior mutability to allow changing the permissions at runtime
#[derive(Clone, Default, Debug)]
pub struct AllowlistWebPermissions(Arc<RwLock<AllowlistWebPermissionsSet>>);
impl AllowlistWebPermissions {
    /// Create a new instance with nothing allowed by default
    #[must_use]
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(AllowlistWebPermissionsSet::default())))
    }

    fn borrow(&self) -> std::sync::RwLockReadGuard<AllowlistWebPermissionsSet> {
        self.0.read().expect("Could not lock permissions")
    }

    fn borrow_mut(&self) -> std::sync::RwLockWriteGuard<AllowlistWebPermissionsSet> {
        self.0.write().expect("Could not lock permissions")
    }

    /// Set the `hrtime` permission
    /// If true, timers will be allowed to use high resolution time
    pub fn set_hrtime(&self, value: bool) {
        self.borrow_mut().hrtime = value;
    }

    /// Set the `exec` permission
    /// If true, FFI execution will be allowed
    pub fn set_exec(&self, value: bool) {
        self.borrow_mut().exec = value;
    }

    /// Set the `read_all` permission
    /// If false all reads will be denied
    pub fn set_read_all(&self, value: bool) {
        self.borrow_mut().read_all = value;
    }

    /// Set the `write_all` permission
    /// If false all writes will be denied
    pub fn set_write_all(&self, value: bool) {
        self.borrow_mut().write_all = value;
    }

    /// Whitelist a path for opening
    /// If `read` is true, the path will be allowed to be opened for reading
    /// If `write` is true, the path will be allowed to be opened for writing
    pub fn allow_open(&self, path: &str, read: bool, write: bool) {
        if read {
            self.borrow_mut().openr_paths.insert(path.to_string());
        }
        if write {
            self.borrow_mut().openw_paths.insert(path.to_string());
        }
    }

    /// Whitelist a URL
    pub fn allow_url(&self, url: &str) {
        self.borrow_mut().url.insert(url.to_string());
    }

    /// Blacklist a URL
    pub fn deny_url(&self, url: &str) {
        self.borrow_mut().url.remove(url);
    }

    /// Whitelist a path for reading
    pub fn allow_read(&self, path: &str) {
        self.borrow_mut().read_paths.insert(path.to_string());
    }

    /// Blacklist a path for reading
    pub fn deny_read(&self, path: &str) {
        self.borrow_mut().read_paths.remove(path);
    }

    /// Whitelist a path for writing
    pub fn allow_write(&self, path: &str) {
        self.borrow_mut().write_paths.insert(path.to_string());
    }

    /// Blacklist a path for writing
    pub fn deny_write(&self, path: &str) {
        self.borrow_mut().write_paths.remove(path);
    }

    /// Whitelist a host
    pub fn allow_host(&self, host: &str) {
        self.borrow_mut().hosts.insert(host.to_string());
    }

    /// Blacklist a host
    pub fn deny_host(&self, host: &str) {
        self.borrow_mut().hosts.remove(host);
    }

    /// Whitelist an environment variable
    pub fn allow_env(&self, var: &str) {
        self.borrow_mut().envs.insert(var.to_string());
    }

    /// Blacklist an environment variable
    pub fn deny_env(&self, var: &str) {
        self.borrow_mut().envs.remove(var);
    }

    /// Whitelist a system operation
    pub fn allow_sys(&self, kind: SystemsPermissionKind) {
        self.borrow_mut().sys.insert(kind);
    }

    /// Blacklist a system operation
    pub fn deny_sys(&self, kind: SystemsPermissionKind) {
        self.borrow_mut().sys.remove(&kind);
    }
}
impl WebPermissions for AllowlistWebPermissions {
    fn allow_hrtime(&self) -> bool {
        self.borrow().hrtime
    }

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        if self.borrow().hosts.contains(host) {
            Ok(())
        } else {
            PermissionDenied::oops(host)?
        }
    }

    fn check_url(&self, url: &deno_core::url::Url, api_name: &str) -> Result<(), PermissionDenied> {
        if self.borrow().url.contains(url.as_str()) {
            Ok(())
        } else {
            PermissionDenied::oops(url)?
        }
    }

    fn check_read<'a>(
        &self,
        p: &'a Path,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDenied> {
        let inst = self.borrow();
        if inst.read_all && inst.read_paths.contains(p.to_str().unwrap()) {
            Ok(Cow::Borrowed(p))
        } else {
            PermissionDenied::oops(p.display())?
        }
    }

    fn check_write<'a>(
        &self,
        p: &'a Path,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDenied> {
        let inst = self.borrow();
        if inst.write_all && inst.write_paths.contains(p.to_str().unwrap()) {
            Ok(Cow::Borrowed(p))
        } else {
            PermissionDenied::oops(p.display())?
        }
    }

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: &'a Path,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>> {
        let path = path.to_str().unwrap();
        if read && !self.borrow().openr_paths.contains(path) {
            return None;
        }
        if write && !self.borrow().openw_paths.contains(path) {
            return None;
        }
        Some(Cow::Borrowed(path.as_ref()))
    }

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionDenied> {
        if self.borrow().read_all {
            Ok(())
        } else {
            PermissionDenied::oops("read_all")?
        }
    }

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        if !self.borrow().read_all {
            return PermissionDenied::oops("read_all")?;
        }
        self.check_read(p, Some(api_name))?;
        Ok(())
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionDenied> {
        if self.borrow().write_all {
            Ok(())
        } else {
            PermissionDenied::oops("write_all")?
        }
    }

    fn check_write_blind(
        &self,
        path: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        self.check_write(Path::new(path), Some(api_name))?;
        Ok(())
    }

    fn check_write_partial(
        &self,
        path: &str,
        api_name: &str,
    ) -> Result<std::path::PathBuf, PermissionDenied> {
        let p = self.check_write(Path::new(path), Some(api_name))?;
        Ok(p.into_owned())
    }

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionDenied> {
        if self.borrow().sys.contains(&kind) {
            Ok(())
        } else {
            PermissionDenied::oops(kind.as_str())?
        }
    }

    fn check_env(&self, var: &str) -> Result<(), PermissionDenied> {
        if self.borrow().envs.contains(var) {
            Ok(())
        } else {
            PermissionDenied::oops(var)?
        }
    }

    fn check_exec(&self) -> Result<(), PermissionDenied> {
        if self.borrow().exec {
            Ok(())
        } else {
            PermissionDenied::oops("ffi")?
        }
    }
}

/// Trait managing the permissions for the web related extensions
/// See [`DefaultWebPermissions`] for a default implementation that allows-all
pub trait WebPermissions: std::fmt::Debug + Send + Sync {
    /// Check if `hrtime` is allowed
    /// If true, timers will be allowed to use high resolution time
    fn allow_hrtime(&self) -> bool;

    /// Check if a URL is allowed to be used by fetch or websocket
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_url(&self, url: &deno_core::url::Url, api_name: &str) -> Result<(), PermissionDenied>;

    /// Check if a path is allowed to be opened by fs
    /// If the path is allowed, the returned path will be used instead
    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: &'a Path,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>>;

    /// Check if a path is allowed to be read by fetch or net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read<'a>(
        &self,
        p: &'a Path,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDenied>;

    /// Check if all paths are allowed to be read by fs
    /// Used by `deno_fs` for `op_fs_symlink`
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionDenied>;

    /// Check if a path is allowed to be read by fs
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDenied>;

    /// Check if a path is allowed to be written to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write<'a>(
        &self,
        p: &'a Path,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionDenied>;

    /// Check if all paths are allowed to be written to by fs
    /// Used by `deno_fs` for `op_fs_symlink`
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionDenied>;

    /// Check if a path is allowed to be written to by fs
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionDenied>;

    /// Check if a path is allowed to be written to by fs
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write_partial(
        &self,
        path: &str,
        api_name: &str,
    ) -> Result<std::path::PathBuf, PermissionDenied>;

    /// Check if a host is allowed to be connected to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionDenied>;

    /// Check if a system operation is allowed
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionDenied>;

    /// Check if an environment variable is allowed to be accessed
    /// Used by remote KV store (`deno_kv`)
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_env(&self, var: &str) -> Result<(), PermissionDenied>;

    /// Check if FFI execution is allowed
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_exec(&self) -> Result<(), PermissionDenied>;
}

macro_rules! impl_sys_permission_kinds {
    ($($kind:ident($name:literal)),+ $(,)?) => {
        /// Knows systems permission checks performed by deno
        ///
        /// This list is updated manually using:
        /// <https://github.com/search?q=repo%3Adenoland%2Fdeno%20check_sys&type=code>
        #[derive(Debug, Clone, PartialEq, Eq, Hash)]
        pub enum SystemsPermissionKind {
            $(
                #[doc = stringify!($kind)]
                $kind,
            )+

            /// A custom permission kind
            Other(String),
        }
        impl SystemsPermissionKind {
            /// Create a new instance from a string
            #[must_use]
            pub fn new(s: &str) -> Self {
                match s {
                    $( $name => Self::$kind, )+
                    _ => Self::Other(s.to_string()),
                }
            }

            /// Get the string representation of the permission
            #[must_use]
            pub fn as_str(&self) -> &str {
                match self {
                    $( Self::$kind => $name, )+
                    Self::Other(s) => &s,
                }
            }
        }
    };
}

impl_sys_permission_kinds!(
    LoadAvg("loadavg"),
    Hostname("hostname"),
    OsRelease("osRelease"),
    Networkinterfaces("networkInterfaces"),
    StatFs("statfs"),
    GetPriority("getPriority"),
    SystemMemoryInfo("systemMemoryInfo"),
    Gid("gid"),
    Uid("uid"),
    OsUptime("osUptime"),
    SetPriority("setPriority"),
    UserInfo("userInfo"),
    GetEGid("getegid"),
    Cpus("cpus"),
    HomeDir("homeDir"),
    Inspector("inspector"),
);

#[derive(Clone, Debug)]
pub struct PermissionsContainer(pub Arc<dyn WebPermissions>);
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
    ) -> Result<(), PermissionCheckError> {
        self.0.check_url(url, api_name)?;
        Ok(())
    }

    fn check_read<'a>(
        &mut self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        let p = self.0.check_read(p, Some(api_name))?;
        Ok(p)
    }
}
impl deno_net::NetPermissions for PermissionsContainer {
    fn check_net<T: AsRef<str>>(
        &mut self,
        host: &(T, Option<u16>),
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_host(host.0.as_ref(), host.1, api_name)?;
        Ok(())
    }

    fn check_read(&mut self, p: &str, api_name: &str) -> Result<PathBuf, PermissionCheckError> {
        let p = self
            .0
            .check_read(Path::new(p), Some(api_name))
            .map(std::borrow::Cow::into_owned)?;
        Ok(p)
    }

    fn check_write(&mut self, p: &str, api_name: &str) -> Result<PathBuf, PermissionCheckError> {
        let p = self
            .0
            .check_write(Path::new(p), Some(api_name))
            .map(std::borrow::Cow::into_owned)?;
        Ok(p)
    }

    fn check_write_path<'a>(
        &mut self,
        p: &'a Path,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        let p = self.0.check_write(p, Some(api_name))?;
        Ok(p)
    }
}
