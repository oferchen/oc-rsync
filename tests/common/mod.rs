// tests/common/mod.rs
#![allow(dead_code)]

use assert_cmd::Command;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::Path;

pub fn oc_cmd() -> Command {
    let mut cmd = Command::cargo_bin("oc-rsync").unwrap();
    cmd.env("LC_ALL", "C").env("LANG", "C");
    cmd
}

pub mod daemon;

pub struct EnvVarGuard {
    key: OsString,
    old: Option<OsString>,
}

pub fn temp_env<K, V>(key: K, value: V) -> EnvVarGuard
where
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
{
    let key_os = key.as_ref().to_os_string();
    let old = env::var_os(&key_os);
    unsafe { env::set_var(&key_os, value) };
    EnvVarGuard { key: key_os, old }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        match self.old.take() {
            Some(val) => unsafe { env::set_var(&self.key, val) },
            None => unsafe { env::remove_var(&self.key) },
        }
    }
}

pub fn with_env_var<K, V, F, R>(key: K, value: V, f: F) -> R
where
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
    F: FnOnce() -> R,
{
    let _guard = temp_env(key, value);
    f()
}

pub fn read_golden(name: &str) -> (Vec<u8>, Vec<u8>, i32) {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/golden")
        .join(name);
    let stdout = fs::read(base.with_extension("stdout")).unwrap_or_default();
    let stderr = fs::read(base.with_extension("stderr")).unwrap_or_default();
    let exit: i32 = fs::read_to_string(base.with_extension("exit"))
        .unwrap_or_else(|_| "0".into())
        .trim()
        .parse()
        .unwrap();
    (stdout, stderr, exit)
}

pub fn normalize_paths(data: &[u8], root: &Path) -> String {
    let text = String::from_utf8_lossy(data).replace('\r', "");
    let root_str = root.to_string_lossy().replace("\\", "/");
    text.replace(&root_str, "TMP")
}

pub fn parse_literal(stats: &str) -> usize {
    for line in stats.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("Literal data: ")
            .or_else(|| line.strip_prefix("Unmatched data: "))
        {
            let num_str = rest.split_whitespace().next().unwrap().replace(",", "");
            return num_str.parse().unwrap();
        }
    }
    panic!("no literal data in stats: {stats}");
}
