use std::{
    borrow::Cow,
    collections::HashSet,
    path::Path,
    sync::{Arc, RwLock},
};

pub use deno_permissions::{CheckedPath, PermissionCheckError, PermissionDeniedError};

pub fn oops(msg: impl std::fmt::Display) -> PermissionCheckError {
    PermissionCheckError::PermissionDenied(PermissionDeniedError {
        access: msg.to_string(),
        name: "web",
    })
}

/// The default permissions manager for the web related extensions
///
/// Allows all operations
#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultWebPermissions;
impl WebPermissions for DefaultWebPermissions {
    fn allow_hrtime(&self) -> bool {
        true
    }

    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>> {
        Some(path)
    }

    fn check_read<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        Ok(p)
    }

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_write<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        Ok(p)
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_write_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_write_partial<'a>(
        &self,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        Ok(path)
    }

    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_vsock(&self, cid: u32, port: u32, api_name: &str) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_env(&self, var: &str) -> Result<(), PermissionCheckError> {
        Ok(())
    }

    fn check_exec(&self) -> Result<(), PermissionCheckError> {
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
    pub vsock: HashSet<(u32, u32)>,
}

/// Permissions manager for the web related extensions
///
/// Allows only operations that are explicitly enabled
///
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
    ///
    /// If true, timers will be allowed to use high resolution time
    pub fn set_hrtime(&self, value: bool) {
        self.borrow_mut().hrtime = value;
    }

    /// Set the `exec` permission
    ///
    /// If true, FFI execution will be allowed
    pub fn set_exec(&self, value: bool) {
        self.borrow_mut().exec = value;
    }

    /// Set the `read_all` permission
    ///
    /// If false all reads will be denied
    pub fn set_read_all(&self, value: bool) {
        self.borrow_mut().read_all = value;
    }

    /// Set the `write_all` permission
    ///
    /// If false all writes will be denied
    pub fn set_write_all(&self, value: bool) {
        self.borrow_mut().write_all = value;
    }

    /// Whitelist a path for opening
    ///
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

    /// Whitelist a virtual socket
    pub fn allow_vsock(&self, cid: u32, port: u32) {
        self.borrow_mut().vsock.insert((cid, port));
    }

    /// Blacklist a virtual socket
    pub fn deny_vsock(&self, cid: u32, port: u32) {
        self.borrow_mut().vsock.remove(&(cid, port));
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
    ) -> Result<(), PermissionCheckError> {
        if self.borrow().hosts.contains(host) {
            Ok(())
        } else {
            Err(oops(host))
        }
    }

    fn check_vsock(&self, cid: u32, port: u32, api_name: &str) -> Result<(), PermissionCheckError> {
        if self.borrow().vsock.contains(&(cid, port)) {
            Ok(())
        } else {
            Err(oops(format!("vsock: {cid}:{port}")))
        }
    }

    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        if self.borrow().url.contains(url.as_str()) {
            Ok(())
        } else {
            Err(oops(url))
        }
    }

    fn check_read<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        let inst = self.borrow();
        if inst.read_all && inst.read_paths.contains(p.to_str().unwrap()) {
            Ok(p)
        } else {
            Err(oops(p.display()))
        }
    }

    fn check_write<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        let inst = self.borrow();
        if inst.write_all && inst.write_paths.contains(p.to_str().unwrap()) {
            Ok(p)
        } else {
            Err(oops(p.display()))
        }
    }

    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>> {
        let path_str = path.to_str().unwrap();
        if read && !self.borrow().openr_paths.contains(path_str) {
            return None;
        }
        if write && !self.borrow().openw_paths.contains(path_str) {
            return None;
        }
        Some(path)
    }

    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionCheckError> {
        if self.borrow().read_all {
            Ok(())
        } else {
            Err(oops("read_all"))
        }
    }

    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        if !self.borrow().read_all {
            return Err(oops("read_all"));
        }
        self.check_read(Cow::Borrowed(p), Some(api_name))?;
        Ok(())
    }

    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionCheckError> {
        if self.borrow().write_all {
            Ok(())
        } else {
            Err(oops("write_all"))
        }
    }

    fn check_write_blind(
        &self,
        path: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.check_write(Cow::Borrowed(path), Some(api_name))?;
        Ok(())
    }

    fn check_write_partial<'a>(
        &self,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, PermissionCheckError> {
        let p = self.check_write(path, Some(api_name))?;
        Ok(p)
    }

    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        if self.borrow().sys.contains(&kind) {
            Ok(())
        } else {
            Err(oops(kind.as_str()))
        }
    }

    fn check_env(&self, var: &str) -> Result<(), PermissionCheckError> {
        if self.borrow().envs.contains(var) {
            Ok(())
        } else {
            Err(oops(var))
        }
    }

    fn check_exec(&self) -> Result<(), PermissionCheckError> {
        if self.borrow().exec {
            Ok(())
        } else {
            Err(oops("ffi"))
        }
    }
}

/// Trait managing the permissions for the web related extensions
///
/// See [`DefaultWebPermissions`] for a default implementation that allows-all
pub trait WebPermissions: std::fmt::Debug + Send + Sync {
    /// Check if `hrtime` is allowed
    ///
    /// If true, timers will be allowed to use high resolution time
    fn allow_hrtime(&self) -> bool;

    /// Check if a URL is allowed to be used by fetch or websocket
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_url(
        &self,
        url: &deno_core::url::Url,
        api_name: &str,
    ) -> Result<(), PermissionCheckError>;

    /// Check if a path is allowed to be opened by fs
    ///
    /// If the path is allowed, the returned path will be used instead
    fn check_open<'a>(
        &self,
        resolved: bool,
        read: bool,
        write: bool,
        path: Cow<'a, Path>,
        api_name: &str,
    ) -> Option<std::borrow::Cow<'a, Path>>;

    /// Check if a path is allowed to be read by fetch or net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionCheckError>;

    /// Check if all paths are allowed to be read by fs
    ///
    /// Used by `deno_fs` for `op_fs_symlink`
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read_all(&self, api_name: Option<&str>) -> Result<(), PermissionCheckError>;

    /// Check if a path is allowed to be read by fs
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_read_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionCheckError>;

    /// Check if a path is allowed to be written to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: Option<&str>,
    ) -> Result<Cow<'a, Path>, PermissionCheckError>;

    /// Check if all paths are allowed to be written to by fs
    ///
    /// Used by `deno_fs` for `op_fs_symlink`
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write_all(&self, api_name: &str) -> Result<(), PermissionCheckError>;

    /// Check if a path is allowed to be written to by fs
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write_blind(
        &self,
        p: &Path,
        display: &str,
        api_name: &str,
    ) -> Result<(), PermissionCheckError>;

    /// Check if a path is allowed to be written to by fs
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_write_partial<'a>(
        &self,
        p: Cow<'a, Path>,
        api_name: &str,
    ) -> Result<Cow<'a, Path>, PermissionCheckError>;

    /// Check if a host is allowed to be connected to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_host(
        &self,
        host: &str,
        port: Option<u16>,
        api_name: &str,
    ) -> Result<(), PermissionCheckError>;

    /// Check if a virtual socket is allowed to be connected to by net
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_vsock(&self, cid: u32, port: u32, api_name: &str) -> Result<(), PermissionCheckError>;

    /// Check if a system operation is allowed
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_sys(
        &self,
        kind: SystemsPermissionKind,
        api_name: &str,
    ) -> Result<(), PermissionCheckError>;

    /// Check if an environment variable is allowed to be accessed
    ///
    /// Used by remote KV store (`deno_kv`)
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_env(&self, var: &str) -> Result<(), PermissionCheckError>;

    /// Check if FFI execution is allowed
    ///
    /// # Errors
    /// If an error is returned, the operation will be denied with the error message as the reason
    fn check_exec(&self) -> Result<(), PermissionCheckError>;
}

macro_rules! impl_sys_permission_kinds {
    ($($kind:ident($name:literal)),+ $(,)?) => {
        /// Knows systems permission checks performed by deno
        ///
        /// This list is updated manually using:
        /// <https://github.com/search?q=repo%3Adenoland%2Fdeno+check_sys%28%22&type=code>
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

    fn check_open<'a>(
        &mut self,
        path: Cow<'a, Path>,
        open_access: deno_permissions::OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        let read = open_access.is_read();
        let write = open_access.is_write();

        let p = self
            .0
            .check_open(true, read, write, path, api_name)
            .ok_or(oops("open"))?;

        Ok(CheckedPath::unsafe_new(p))
    }

    fn check_net_vsock(
        &mut self,
        cid: u32,
        port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_vsock(cid, port, api_name)?;
        Ok(())
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

    fn check_open<'a>(
        &mut self,
        path: Cow<'a, Path>,
        open_access: deno_permissions::OpenAccessKind,
        api_name: &str,
    ) -> Result<CheckedPath<'a>, PermissionCheckError> {
        let read = open_access.is_read();
        let write = open_access.is_write();

        let p = self
            .0
            .check_open(true, read, write, path, api_name)
            .ok_or(oops("open"))?;

        Ok(CheckedPath::unsafe_new(p))
    }

    fn check_vsock(
        &mut self,
        cid: u32,
        port: u32,
        api_name: &str,
    ) -> Result<(), PermissionCheckError> {
        self.0.check_vsock(cid, port, api_name)?;
        Ok(())
    }
}
