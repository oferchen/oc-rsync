// tests/common/mod.rs
#![allow(dead_code)]

use std::env;
use std::ffi::{OsStr, OsString};

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
    unsafe {
        env::set_var(&key_os, value);
    }
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
