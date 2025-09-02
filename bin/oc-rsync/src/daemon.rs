// bin/oc-rsync/src/daemon.rs
use oc_rsync_cli::version;
use std::ffi::OsString;
use std::process::Command;

fn main() {
    let version = OsString::from("--version");
    let version_short = OsString::from("-V");
    let quiet = OsString::from("--quiet");
    let quiet_short = OsString::from("-q");
    if std::env::args_os().any(|a| a == version || a == version_short) {
        if !std::env::args_os().any(|a| a == quiet || a == quiet_short) {
            println!("{}", version::render_version_lines().join("\n"));
        }
        return;
    }

    let oc_rsync = std::env::var_os("OC_RSYNC_BIN")
        .or_else(|| option_env!("CARGO_BIN_EXE_oc-rsync").map(OsString::from))
        .unwrap_or_else(|| OsString::from("oc-rsync"));
    let status = Command::new(&oc_rsync)
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
