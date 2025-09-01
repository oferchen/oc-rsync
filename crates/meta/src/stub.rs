// crates/meta/src/stub.rs
#[cfg(unix)]
include!("unix.rs");

#[cfg(not(unix))]
mod non_unix {
    use filetime::FileTime;
    #[cfg(feature = "acl")]
    use posix_acl::ACLEntry;
    #[cfg(feature = "xattr")]
    use std::ffi::OsString;
    use std::io;
    use std::path::Path;
    use std::sync::Arc;

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
    }

    impl std::fmt::Debug for Options {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
        #[cfg(feature = "xattr")]
        pub xattrs: Vec<(OsString, Vec<u8>)>,
        #[cfg(feature = "acl")]
        pub acl: Vec<ACLEntry>,
        #[cfg(feature = "acl")]
        pub default_acl: Vec<ACLEntry>,
    }

    impl Metadata {
        pub fn from_path(_path: &Path, _opts: Options) -> io::Result<Self> {
            Ok(Metadata {
                uid: 0,
                gid: 0,
                mode: 0,
                mtime: FileTime::from_unix_time(0, 0),
                atime: None,
                crtime: None,
                #[cfg(feature = "xattr")]
                xattrs: Vec::new(),
                #[cfg(feature = "acl")]
                acl: Vec::new(),
                #[cfg(feature = "acl")]
                default_acl: Vec::new(),
            })
        }

        pub fn apply(&self, _path: &Path, _opts: Options) -> io::Result<()> {
            Ok(())
        }
    }
}

#[cfg(feature = "xattr")]
pub fn store_fake_super(_path: &Path, _uid: u32, _gid: u32, _mode: u32) {}

#[cfg(not(unix))]
pub use non_unix::*;
