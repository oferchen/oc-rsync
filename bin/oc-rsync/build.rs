// bin/oc-rsync/build.rs
use std::{env, fs, path::Path};

fn main() {
    let upstream = env::var("RSYNC_UPSTREAM_VER").unwrap_or_else(|_| "unknown".to_string());
    let revision = env::var("BUILD_REVISION").unwrap_or_else(|_| "unknown".to_string());
    let official = env::var("OFFICIAL_BUILD").unwrap_or_else(|_| "unofficial".to_string());

    println!("cargo:rustc-env=RSYNC_UPSTREAM_VER={upstream}");
    println!("cargo:rustc-env=BUILD_REVISION={revision}");
    println!("cargo:rustc-env=OFFICIAL_BUILD={official}");

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("missing manifest dir");
    let docs_dir = Path::new(&manifest_dir).join("../../docs");
    let _ = fs::create_dir_all(&docs_dir);
    let info_path = docs_dir.join("build_info.md");
    let contents = format!(
        "rsync upstream version: {upstream}\nbuild revision: {revision}\nofficial build: {official}\n"
    );
    fs::write(info_path, contents).expect("failed to write build_info.md");

    println!("cargo:rerun-if-env-changed=RSYNC_UPSTREAM_VER");
    println!("cargo:rerun-if-env-changed=BUILD_REVISION");
    println!("cargo:rerun-if-env-changed=OFFICIAL_BUILD");
}
