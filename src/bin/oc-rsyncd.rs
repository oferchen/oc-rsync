// src/bin/oc-rsyncd.rs
use std::ffi::OsString;
use std::process::Command;

use oc_rsync_core as _;
#[cfg(unix)]
use std::os::unix::process::CommandExt;

fn main() {
    let oc_rsync = std::env::var_os("OC_RSYNC_BIN")
        .or_else(|| option_env!("CARGO_BIN_EXE_oc-rsync").map(OsString::from))
        .unwrap_or_else(|| OsString::from("oc-rsync"));
    let user_args = std::env::args_os().skip(1);
    #[cfg(windows)]
    {
        let status = Command::new(&oc_rsync)
            .arg("--daemon")
            .args(user_args)
            .status()
            .unwrap_or_else(|e| {
                eprintln!("{e}");
                std::process::exit(1);
            });
        if let Some(code) = status.code() {
            std::process::exit(code);
        } else {
            std::process::exit(1);
        }
    }
    #[cfg(unix)]
    {
        let err = Command::new(&oc_rsync)
            .arg("--daemon")
            .args(user_args)
            .exec();
        eprintln!("{err}");
        std::process::exit(1);
    }
}
