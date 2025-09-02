// bin/oc-rsyncd/src/main.rs
use std::ffi::OsString;
use std::process::Command;

fn main() {
    let oc_rsync = std::env::var_os("OC_RSYNC_BIN")
        .or_else(|| option_env!("CARGO_BIN_EXE_oc-rsync").map(OsString::from))
        .unwrap_or_else(|| OsString::from("oc-rsync"));

    let status = Command::new(oc_rsync)
        .arg("--daemon")
        .args(std::env::args_os().skip(1))
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
