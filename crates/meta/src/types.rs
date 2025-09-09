// crates/meta/src/types.rs
use filetime::FileTime;
#[cfg(all(unix, feature = "acl"))]
use posix_acl::ACLEntry;
#[cfg(unix)]
use std::ffi::{OsStr, OsString};
use std::fmt;
#[cfg(unix)]
use std::rc::Rc;
use std::sync::Arc;
#[cfg(unix)]
pub type XattrFilter = Rc<dyn Fn(&OsStr) -> bool>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodTarget {
    All,
    File,
    Dir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChmodOp {
    Add,
    Remove,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chmod {
    pub target: ChmodTarget,
    pub op: ChmodOp,
    pub mask: u32,
    pub bits: u32,
    pub conditional: bool,
}

#[derive(Clone, Default)]
pub struct Options {
    pub xattrs: bool,
    pub acl: bool,
    pub fake_super: bool,
    pub super_user: bool,
    pub numeric_ids: bool,
    pub chmod: Option<Vec<Chmod>>,
    pub owner: bool,
    pub group: bool,
    pub perms: bool,
    pub executability: bool,
    pub times: bool,
    pub atimes: bool,
    pub crtimes: bool,
    pub omit_dir_times: bool,
    pub omit_link_times: bool,
    pub uid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
    pub gid_map: Option<Arc<dyn Fn(u32) -> u32 + Send + Sync>>,
    #[cfg(unix)]
    pub xattr_filter: Option<XattrFilter>,
    #[cfg(unix)]
    pub xattr_filter_delete: Option<XattrFilter>,
}

impl fmt::Debug for Options {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Options")
            .field("xattrs", &self.xattrs)
            .field("acl", &self.acl)
            .field("fake_super", &self.fake_super)
            .field("super_user", &self.super_user)
            .field("numeric_ids", &self.numeric_ids)
            .field("chmod", &self.chmod)
            .field("owner", &self.owner)
            .field("group", &self.group)
            .field("perms", &self.perms)
            .field("executability", &self.executability)
            .field("times", &self.times)
            .field("atimes", &self.atimes)
            .field("crtimes", &self.crtimes)
            .field("omit_dir_times", &self.omit_dir_times)
            .field("omit_link_times", &self.omit_link_times)
            .field("uid_map", &self.uid_map.is_some())
            .field("gid_map", &self.gid_map.is_some())
            .field("xattr_filter", &{
                #[cfg(unix)]
                {
                    self.xattr_filter.is_some()
                }
                #[cfg(not(unix))]
                {
                    false
                }
            })
            .field("xattr_filter_delete", &{
                #[cfg(unix)]
                {
                    self.xattr_filter_delete.is_some()
                }
                #[cfg(not(unix))]
                {
                    false
                }
            })
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Metadata {
    pub uid: u32,
    pub gid: u32,
    pub mode: u32,
    pub mtime: FileTime,
    pub atime: Option<FileTime>,
    pub crtime: Option<FileTime>,
    #[cfg(unix)]
    pub xattrs: Vec<(OsString, Vec<u8>)>,
    #[cfg(all(unix, feature = "acl"))]
    pub acl: Vec<ACLEntry>,
    #[cfg(all(unix, feature = "acl"))]
    pub default_acl: Vec<ACLEntry>,
}
