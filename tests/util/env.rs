// tests/util/env.rs
use std::ffi::{OsStr, OsString};

pub fn with_env_var<K, V, F, R>(key: K, value: V, f: F) -> R
where
    K: AsRef<OsStr>,
    V: AsRef<OsStr>,
    F: FnOnce() -> R,
{
    let key = key.as_ref().to_os_string();
    let prev = std::env::var_os(&key);
    unsafe { std::env::set_var(&key, value) };
    struct Guard {
        key: OsString,
        prev: Option<OsString>,
    }
    impl Drop for Guard {
        fn drop(&mut self) {
            if let Some(val) = &self.prev {
                unsafe { std::env::set_var(&self.key, val) };
            } else {
                unsafe { std::env::remove_var(&self.key) };
            }
        }
    }
    let guard = Guard {
        key: key.clone(),
        prev,
    };
    let result = f();
    drop(guard);
    result
}
