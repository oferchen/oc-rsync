// crates/meta/src/stub.rs
#[cfg(unix)]
include!("unix/mod.rs");

#[cfg(not(unix))]
mod non_unix {
    use std::io;
    use std::path::Path;

    use crate::{Metadata, Options};

    #[derive(Default, Debug)]
    pub struct HardLinks;

    impl HardLinks {
        pub fn register(&mut self, _id: u64, _path: &Path) -> bool {
            unimplemented!("hard links are not supported on this platform")
        }

        pub fn finalize(&mut self) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "hard links are not supported on this platform",
            ))
        }
    }

    pub fn hard_link_id(_dev: u64, _ino: u64) -> u64 {
        unimplemented!("hard links are not supported on this platform")
    }

    pub fn read_acl(_path: &Path, _fake_super: bool) -> io::Result<(Vec<()>, Vec<()>)> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ACLs are not supported on this platform",
        ))
    }

    pub fn write_acl(
        _path: &Path,
        _acl: &[()],
        _default_acl: &[()],
        _fake_super: bool,
        _super_user: bool,
    ) -> io::Result<()> {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "ACLs are not supported on this platform",
        ))
    }

    pub fn store_fake_super(_path: &Path, _uid: u32, _gid: u32, _mode: u32) {
        unimplemented!("fake super is not supported on this platform")
    }

    impl Metadata {
        pub fn from_path(_path: &Path, _opts: Options) -> io::Result<Self> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "metadata operations are not supported on this platform",
            ))
        }

        pub fn apply(&self, _path: &Path, _opts: Options) -> io::Result<()> {
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "metadata operations are not supported on this platform",
            ))
        }
    }
}

#[cfg(not(unix))]
pub use non_unix::*;
