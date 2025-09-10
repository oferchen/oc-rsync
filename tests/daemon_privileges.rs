// tests/daemon_privileges.rs
#![cfg(all(unix, feature = "root"))]

use daemon::{chroot_and_drop_privileges, drop_privileges};
use serial_test::serial;
use std::io;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

mod common;
use common::oc_cmd;

#[test]
#[serial]
#[ignore = "requires root privileges"]
fn chroot_drops_privileges() {
    use nix::sys::wait::waitpid;
    use nix::unistd::{ForkResult, fork, getegid, geteuid};

    let _ = oc_cmd();
    let dir = tempdir().unwrap();
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let status = waitpid(child, None).unwrap();
            assert!(matches!(status, nix::sys::wait::WaitStatus::Exited(_, 0)));
        }
        Ok(ForkResult::Child) => {
            let _ctx = chroot_and_drop_privileges(dir.path(), 1, 1, true).unwrap();
            assert_eq!(std::env::current_dir().unwrap(), PathBuf::from("/"));
            assert_eq!(geteuid().as_raw(), 1);
            assert_eq!(getegid().as_raw(), 1);
            std::process::exit(0);
        }
        Err(_) => panic!("fork failed"),
    }
}

#[test]
#[serial]
#[ignore = "requires root privileges"]
fn chroot_requires_root() {
    use nix::sys::wait::waitpid;
    use nix::unistd::{ForkResult, fork, geteuid};

    let _ = oc_cmd();
    let dir = tempdir().unwrap();
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let status = waitpid(child, None).unwrap();
            assert!(matches!(status, nix::sys::wait::WaitStatus::Exited(_, 0)));
        }
        Ok(ForkResult::Child) => {
            drop_privileges(1, 1).unwrap();
            let err = chroot_and_drop_privileges(dir.path(), 1, 1, true)
                .err()
                .unwrap();
            assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
            std::process::exit(0);
        }
        Err(_) => panic!("fork failed"),
    }
}

#[test]
#[serial]
#[ignore = "requires root privileges"]
fn chroot_and_drop_privileges_rejects_missing_dir() {
    let _ = oc_cmd();
    let missing = Path::new("/does/not/exist");
    let err = chroot_and_drop_privileges(missing, 0, 0, true)
        .err()
        .unwrap();
    assert_eq!(err.kind(), io::ErrorKind::NotFound);
}

#[test]
#[serial]
#[ignore = "requires root privileges"]
fn drop_privileges_requires_root() {
    use nix::sys::wait::waitpid;
    use nix::unistd::{ForkResult, fork};

    let _ = oc_cmd();
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let status = waitpid(child, None).unwrap();
            assert!(matches!(status, nix::sys::wait::WaitStatus::Exited(_, 0)));
        }
        Ok(ForkResult::Child) => {
            drop_privileges(1, 1).unwrap();
            let err = drop_privileges(2, 2).unwrap_err();
            assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
            std::process::exit(0);
        }
        Err(_) => panic!("fork failed"),
    }
}

#[test]
#[serial]
#[ignore = "requires root privileges"]
fn drop_privileges_changes_ids() {
    use nix::sys::wait::waitpid;
    use nix::unistd::{ForkResult, fork, getegid, geteuid};

    let _ = oc_cmd();
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let status = waitpid(child, None).unwrap();
            assert!(matches!(status, nix::sys::wait::WaitStatus::Exited(_, 0)));
        }
        Ok(ForkResult::Child) => {
            drop_privileges(1, 1).unwrap();
            assert_eq!(geteuid().as_raw(), 1);
            assert_eq!(getegid().as_raw(), 1);
            std::process::exit(0);
        }
        Err(_) => panic!("fork failed"),
    }
}
