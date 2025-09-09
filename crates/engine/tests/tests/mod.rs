// crates/engine/tests/tests/mod.rs
#![allow(dead_code)]

use std::fs;

use nix::unistd::Uid;
use tempfile::tempdir;

#[cfg(feature = "acl")]
use posix_acl::{ACL_READ, PosixACL, Qualifier};

#[cfg(target_os = "linux")]
use caps::{CapSet, Capability};

/// Capabilities or features required by certain tests.
#[derive(Clone, Copy)]
pub enum CapabilityCheck {
    /// Ability to change file ownership (CAP_CHOWN or root).
    CapChown,
    /// Ability to create device nodes (CAP_MKNOD or root).
    CapMknod,
    /// Support for extended attributes on the filesystem.
    Xattrs,
    /// Support for POSIX ACLs on the filesystem.
    Acls,
}

/// Returns `true` if the current environment supports the requested capability.
/// Prints a message and returns `false` if the capability is missing.
pub fn requires_capability(cap: CapabilityCheck) -> bool {
    match cap {
        CapabilityCheck::CapChown => {
            if Uid::effective().is_root() {
                return true;
            }
            #[cfg(target_os = "linux")]
            {
                if caps::has_cap(None, CapSet::Effective, Capability::CAP_CHOWN).unwrap_or(false) {
                    return true;
                }
                println!("Skipping test: requires CAP_CHOWN");
                return false;
            }
            #[cfg(not(target_os = "linux"))]
            {
                println!("Skipping test: requires root to change ownership");
                return false;
            }
        }
        CapabilityCheck::CapMknod => {
            if Uid::effective().is_root() {
                return true;
            }
            #[cfg(target_os = "linux")]
            {
                if caps::has_cap(None, CapSet::Effective, Capability::CAP_MKNOD).unwrap_or(false) {
                    return true;
                }
                println!("Skipping test: requires CAP_MKNOD");
                return false;
            }
            #[cfg(not(target_os = "linux"))]
            {
                println!("Skipping test: requires root to create device nodes");
                return false;
            }
        }
        CapabilityCheck::Xattrs => {
            #[cfg(feature = "xattr")]
            {
                let tmp = tempdir().unwrap();
                let file = tmp.path().join("f");
                fs::write(&file, b"hi").unwrap();
                if xattr::set(&file, "user.test", b"1").is_ok() {
                    return true;
                }
                println!("Skipping test: xattrs not supported");
                return false;
            }
            #[cfg(not(feature = "xattr"))]
            {
                println!("Skipping test: built without xattr support");
                return false;
            }
        }
        CapabilityCheck::Acls => {
            #[cfg(all(unix, feature = "acl"))]
            {
                let tmp = tempdir().unwrap();
                let file = tmp.path().join("f");
                fs::write(&file, b"hi").unwrap();
                match PosixACL::read_acl(&file) {
                    Ok(mut acl) => {
                        acl.set(Qualifier::User(12345), ACL_READ);
                        if acl.write_acl(&file).is_ok() {
                            return true;
                        }
                        println!("Skipping test: ACLs not supported");
                        false
                    }
                    Err(_) => {
                        println!("Skipping test: ACLs not supported");
                        false
                    }
                }
            }
            #[cfg(not(all(unix, feature = "acl")))]
            {
                println!("Skipping test: built without ACL support");
                false
            }
        }
    }
}
