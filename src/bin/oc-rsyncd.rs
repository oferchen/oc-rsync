// src/bin/oc-rsyncd.rs
use std::ffi::OsString;
use std::process::Command;

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if oc_rsync_cli::print_version_if_requested(args.iter().cloned()) {
        return;
    }

    let oc_rsync = std::env::var_os("OC_RSYNC_BIN")
        .or_else(|| option_env!("CARGO_BIN_EXE_oc-rsync").map(OsString::from))
        .unwrap_or_else(|| OsString::from("oc-rsync"));
    let status = Command::new(&oc_rsync)
        .arg("--daemon")
        .args(&args[1..])
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
