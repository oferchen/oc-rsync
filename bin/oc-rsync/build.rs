// bin/oc-rsync/build.rs
use std::{env, fs, path::Path};

const UPSTREAM_VERSION: &str = "3.4.1";
const UPSTREAM_PROTOCOLS: &[u32] = &[32, 31, 30, 29];

fn main() {
    let revision = env::var("BUILD_REVISION").unwrap_or_else(|_| "unknown".to_string());
    let official = env::var("OFFICIAL_BUILD").unwrap_or_else(|_| "unofficial".to_string());

    let protocols = UPSTREAM_PROTOCOLS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");

    println!("cargo:rustc-env=UPSTREAM_VERSION={UPSTREAM_VERSION}");
    println!("cargo:rustc-env=UPSTREAM_PROTOCOLS={protocols}");
    println!("cargo:rustc-env=BUILD_REVISION={revision}");
    println!("cargo:rustc-env=OFFICIAL_BUILD={official}");

    let out_dir = env::var("OUT_DIR").expect("missing OUT_DIR");
    let info_path = Path::new(&out_dir).join("build_info.md");
    let contents = format!(
        "rsync upstream version: {UPSTREAM_VERSION}\nbuild revision: {revision}\nofficial build: {official}\n",
    );
    fs::write(&info_path, contents).expect("failed to write build_info.md");
    println!("cargo:rustc-env=BUILD_INFO_PATH={}", info_path.display());

    println!("cargo:rerun-if-env-changed=BUILD_REVISION");
    println!("cargo:rerun-if-env-changed=OFFICIAL_BUILD");
}
