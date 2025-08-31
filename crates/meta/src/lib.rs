#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::*;

// Re-export device number helpers so consumers can construct and
// deconstruct `dev_t` values without depending on `nix` directly.
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use nix::sys::stat::{makedev, major, minor};

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
mod stub;
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub use stub::*;

mod parse;
pub use parse::*;

#[cfg(feature = "acl")]
pub use posix_acl::{ACLEntry, PosixACL, Qualifier, ACL_EXECUTE, ACL_READ, ACL_RWX, ACL_WRITE};
