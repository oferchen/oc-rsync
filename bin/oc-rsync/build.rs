use std::process::Command;

fn main() {
    let upstream = std::env::var("OC_RSYNC_UPSTREAM")
        .or_else(|_| std::env::var("UPSTREAM_VERSION"))
        .unwrap_or_else(|_| "unknown".to_string());
    println!("cargo:rustc-env=OC_RSYNC_UPSTREAM={upstream}");

    let git = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=OC_RSYNC_GIT={git}");

    let official = std::env::var("OC_RSYNC_OFFICIAL").unwrap_or_else(|_| "unofficial".to_string());
    println!("cargo:rustc-env=OC_RSYNC_OFFICIAL={official}");

    println!("cargo:rerun-if-changed=.git/HEAD");
}
