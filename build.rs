// build.rs

use std::{env, fs, path::Path};
use time::OffsetDateTime;

const UPSTREAM_VERSION: &str = "3.4.1";
const UPSTREAM_PROTOCOLS: &[u32] = &[32, 31, 30, 29];
const BRANDING_VARS: &[&str] = &[
    "OC_RSYNC_BRAND_NAME",
    "OC_RSYNC_BRAND_TAGLINE",
    "OC_RSYNC_BRAND_VERSION",
    "OC_RSYNC_BRAND_CREDITS",
    "OC_RSYNC_BRAND_HEADER",
    "OC_RSYNC_BRAND_FOOTER",
];

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

    for key in BRANDING_VARS {
        if let Ok(val) = env::var(key) {
            println!("cargo:rustc-env={key}={val}");
        }
        println!("cargo:rerun-if-env-changed={key}");
    }

    let year =
        env::var("CURRENT_YEAR").unwrap_or_else(|_| OffsetDateTime::now_utc().year().to_string());
    println!("cargo:rustc-env=CURRENT_YEAR={year}");
    println!("cargo:rerun-if-env-changed=CURRENT_YEAR");

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
