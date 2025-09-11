// crates/daemon/src/config/model.rs

use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::{Arc, atomic::AtomicUsize};
use std::time::Duration;

use transport::AddressFamily;

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub path: PathBuf,
    pub comment: Option<String>,
    pub hosts_allow: Vec<String>,
    pub hosts_deny: Vec<String>,
    pub auth_users: Vec<String>,
    pub secrets_file: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub use_chroot: bool,
    pub numeric_ids: bool,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub read_only: bool,
    pub write_only: bool,
    pub list: bool,
    pub max_connections: Option<u32>,
    pub refuse_options: Vec<String>,
    pub connections: Arc<AtomicUsize>,
}

impl Clone for Module {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            path: self.path.clone(),
            comment: self.comment.clone(),
            hosts_allow: self.hosts_allow.clone(),
            hosts_deny: self.hosts_deny.clone(),
            auth_users: self.auth_users.clone(),
            secrets_file: self.secrets_file.clone(),
            timeout: self.timeout,
            use_chroot: self.use_chroot,
            numeric_ids: self.numeric_ids,
            uid: self.uid,
            gid: self.gid,
            read_only: self.read_only,
            write_only: self.write_only,
            list: self.list,
            max_connections: self.max_connections,
            refuse_options: self.refuse_options.clone(),
            connections: Arc::clone(&self.connections),
        }
    }
}

impl Default for Module {
    fn default() -> Self {
        Self {
            name: String::new(),
            path: PathBuf::new(),
            comment: None,
            hosts_allow: Vec::new(),
            hosts_deny: Vec::new(),
            auth_users: Vec::new(),
            secrets_file: None,
            timeout: None,
            use_chroot: true,
            numeric_ids: false,
            uid: None,
            gid: None,
            read_only: true,
            write_only: false,
            list: true,
            max_connections: None,
            refuse_options: Vec::new(),
            connections: Arc::new(AtomicUsize::new(0)),
        }
    }
}

pub struct ModuleBuilder {
    pub(crate) inner: Module,
}

impl ModuleBuilder {
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self {
            inner: Module {
                name: name.into(),
                path: path.into(),
                ..Module::default()
            },
        }
    }

    pub fn comment(mut self, comment: impl Into<String>) -> Self {
        self.inner.comment = Some(comment.into());
        self
    }

    pub fn hosts_allow(mut self, hosts: Vec<String>) -> Self {
        self.inner.hosts_allow = hosts;
        self
    }

    pub fn hosts_deny(mut self, hosts: Vec<String>) -> Self {
        self.inner.hosts_deny = hosts;
        self
    }

    pub fn auth_users(mut self, users: Vec<String>) -> Self {
        self.inner.auth_users = users;
        self
    }

    pub fn secrets_file(mut self, path: PathBuf) -> Self {
        self.inner.secrets_file = Some(path);
        self
    }

    pub fn timeout(mut self, timeout: Option<Duration>) -> Self {
        self.inner.timeout = timeout;
        self
    }

    pub fn use_chroot(mut self, use_chroot: bool) -> Self {
        self.inner.use_chroot = use_chroot;
        self
    }

    pub fn numeric_ids(mut self, numeric: bool) -> Self {
        self.inner.numeric_ids = numeric;
        self
    }

    pub fn uid(mut self, uid: u32) -> Self {
        self.inner.uid = Some(uid);
        self
    }

    pub fn gid(mut self, gid: u32) -> Self {
        self.inner.gid = Some(gid);
        self
    }

    pub fn read_only(mut self, read_only: bool) -> Self {
        self.inner.read_only = read_only;
        self
    }

    pub fn write_only(mut self, write_only: bool) -> Self {
        self.inner.write_only = write_only;
        self
    }

    pub fn list(mut self, list: bool) -> Self {
        self.inner.list = list;
        self
    }

    pub fn max_connections(mut self, max: u32) -> Self {
        self.inner.max_connections = Some(max);
        self
    }

    pub fn refuse_options(mut self, refuse: Vec<String>) -> Self {
        self.inner.refuse_options = refuse;
        self
    }

    pub fn build(self) -> Module {
        self.inner
    }
}

impl Module {
    pub fn builder(name: impl Into<String>, path: impl Into<PathBuf>) -> ModuleBuilder {
        ModuleBuilder::new(name, path)
    }
}

#[derive(Debug, Default, Clone)]
pub struct DaemonArgs {
    pub address: Option<IpAddr>,
    pub port: u16,
    pub family: Option<AddressFamily>,
}

#[derive(Debug, Default, Clone)]
pub struct DaemonConfig {
    pub address: Option<IpAddr>,
    pub address6: Option<IpAddr>,
    pub port: Option<u16>,
    pub hosts_allow: Vec<String>,
    pub hosts_deny: Vec<String>,
    pub motd_file: Option<PathBuf>,
    pub log_file: Option<PathBuf>,
    pub pid_file: Option<PathBuf>,
    pub lock_file: Option<PathBuf>,
    pub secrets_file: Option<PathBuf>,
    pub timeout: Option<Duration>,
    pub use_chroot: Option<bool>,
    pub numeric_ids: Option<bool>,
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    pub read_only: Option<bool>,
    pub write_only: Option<bool>,
    pub list: Option<bool>,
    pub max_connections: Option<usize>,
    pub refuse_options: Vec<String>,
    pub modules: Vec<Module>,
}
