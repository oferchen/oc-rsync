// tests/daemon_acls/mod.rs
#![cfg(all(unix, feature = "acl"))]

use assert_cmd::Command;
use serial_test::serial;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

use meta::{acls_supported, xattrs_supported};
use posix_acl::{ACL_READ, ACL_WRITE, PosixACL, Qualifier};
mod common;
#[cfg(feature = "root")]
use common::daemon::spawn_rsync_daemon;
use common::daemon::{spawn_daemon, wait_for_daemon};

macro_rules! read_acl_or_skip {
    ($path:expr) => {
        match PosixACL::read_acl($path) {
            Ok(acl) => acl,
            Err(err) => {
                eprintln!("skipping: {}", err);
                return;
            }
        }
    };
}

macro_rules! write_acl_or_skip {
    ($acl:expr, $path:expr) => {
        if let Err(err) = $acl.write_acl($path) {
            eprintln!("skipping: {}", err);
            return;
        }
    };
}

macro_rules! read_default_acl_or_skip {
    ($path:expr) => {
        match PosixACL::read_default_acl($path) {
            Ok(acl) => acl,
            Err(err) => {
                eprintln!("skipping: {}", err);
                return;
            }
        }
    };
}

macro_rules! write_default_acl_or_skip {
    ($acl:expr, $path:expr) => {
        if let Err(err) = $acl.write_default_acl($path) {
            eprintln!("skipping: {}", err);
            return;
        }
    };
}

pub(crate) fn setup_acl_dirs<F>(populate: F) -> (tempfile::TempDir, PathBuf, PathBuf)
where
    F: FnOnce(&Path),
{
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv = tmp.path().join("srv");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv).unwrap();
    populate(&src);
    (tmp, src, srv)
}

#[cfg(feature = "root")]
pub(crate) fn sync_daemon_acls_server() -> Option<(tempfile::TempDir, PathBuf, PathBuf)> {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();

    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = match PosixACL::read_acl(&src_file) {
        Ok(acl) => acl,
        Err(err) => {
            eprintln!("skipping: {}", err);
            return None;
        }
    };
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    if let Err(err) = acl.write_acl(&src_file) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    if let Err(err) = dacl.write_default_acl(&src) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let mut daemon_oc = spawn_daemon(&srv_oc);
    let port_oc = daemon_oc.port;
    wait_for_daemon(&mut daemon_oc);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_oc}/mod")])
        .assert()
        .success();
    let mut daemon_rs = spawn_rsync_daemon(&srv_rs, "");
    let port_rs = daemon_rs.port;
    wait_for_daemon(&mut daemon_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();

    Some((tmp, srv_oc, srv_rs))
}

#[cfg(feature = "root")]
pub(crate) fn sync_daemon_acls_client() -> Option<(tempfile::TempDir, PathBuf, PathBuf)> {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src");
    let srv_oc = tmp.path().join("srv_oc");
    let srv_rs = tmp.path().join("srv_rs");
    fs::create_dir_all(&src).unwrap();
    fs::create_dir_all(&srv_oc).unwrap();
    fs::create_dir_all(&srv_rs).unwrap();

    let src_file = src.join("file");
    fs::write(&src_file, b"hi").unwrap();

    let mut acl = match PosixACL::read_acl(&src_file) {
        Ok(acl) => acl,
        Err(err) => {
            eprintln!("skipping: {}", err);
            return None;
        }
    };
    acl.set(Qualifier::User(12345), ACL_READ);
    acl.set(Qualifier::User(23456), ACL_WRITE);
    if let Err(err) = acl.write_acl(&src_file) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let mut dacl = PosixACL::new(0o755);
    dacl.set(Qualifier::User(12345), ACL_READ);
    if let Err(err) = dacl.write_default_acl(&src) {
        eprintln!("skipping: {}", err);
        return None;
    }

    let mut daemon_oc = spawn_rsync_daemon(&srv_oc, "");
    let port_oc = daemon_oc.port;
    wait_for_daemon(&mut daemon_oc);
    let src_arg = format!("{}/", src.display());
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--acls",
            &src_arg,
            &format!("rsync://127.0.0.1:{port_oc}/mod"),
        ])
        .assert()
        .success();
    let mut daemon_rs = spawn_rsync_daemon(&srv_rs, "");
    let port_rs = daemon_rs.port;
    wait_for_daemon(&mut daemon_rs);
    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args(["-AX", &src_arg, &format!("rsync://127.0.0.1:{port_rs}/mod")])
        .assert()
        .success();

    Some((tmp, srv_oc, srv_rs))
}

mod existing_default_acl;
mod inheritance;
mod preservation;
mod removal;
