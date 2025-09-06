// build.rs

use std::{env, fs, path::Path, path::PathBuf};

use time::OffsetDateTime;

const UPSTREAM_VERSION: &str = "3.4.1";
const SUPPORTED_PROTOCOLS: &[u32] = &[32, 31, 30];
const BRANDING_VARS: &[&str] = &[
    "OC_RSYNC_NAME",
    "OC_RSYNC_VERSION",
    "OC_RSYNC_COPYRIGHT",
    "OC_RSYNC_URL",
];

fn main() {
    let revision = env::var("BUILD_REVISION").unwrap_or_else(|_| "unknown".to_string());
    let official = env::var("OFFICIAL_BUILD").unwrap_or_else(|_| "unofficial".to_string());

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ACL");
    println!("cargo:rerun-if-env-changed=LIBACL_PATH");
    println!("cargo:rerun-if-env-changed=LD_LIBRARY_PATH");
    println!("cargo:rerun-if-env-changed=LIBRARY_PATH");

    if cfg!(unix) {
        if env::var_os("CARGO_FEATURE_ACL").is_some() {
            match pkg_config::Config::new().probe("libacl") {
                Ok(_) => {}
                Err(_) => {
                    let mut search_dirs: Vec<PathBuf> = Vec::new();

                    if let Ok(val) = env::var("LIBACL_PATH") {
                        search_dirs.extend(val.split(':').map(PathBuf::from));
                    }
                    for var in ["LD_LIBRARY_PATH", "LIBRARY_PATH"] {
                        if let Ok(val) = env::var(var) {
                            search_dirs.extend(val.split(':').map(PathBuf::from));
                        }
                    }
                    search_dirs.extend(
                        [
                            "/usr/lib",
                            "/usr/lib64",
                            "/usr/local/lib",
                            "/usr/local/lib64",
                            "/lib",
                            "/lib64",
                        ]
                        .into_iter()
                        .map(PathBuf::from),
                    );

                    let lib_dir = search_dirs.iter().find(|dir| {
                        dir.join("libacl.so").exists() || dir.join("libacl.a").exists()
                    });
                    if let Some(dir) = lib_dir {
                        println!("cargo:rustc-link-lib=acl");
                        println!("cargo:rustc-link-search=native={}", dir.display());
                    } else {
                        println!("cargo:warning=libacl not found; ACL support will be disabled");
                        println!("cargo:rustc-cfg=libacl_missing");
                    }
                }
            }
        }
    } else if env::var_os("CARGO_FEATURE_ACL").is_some() {
        println!("cargo:warning=libacl not found; ACL support will be disabled");
        println!("cargo:rustc-cfg=libacl_missing");
    }

    let protocols = SUPPORTED_PROTOCOLS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");

    println!("cargo:rustc-env=UPSTREAM_VERSION={UPSTREAM_VERSION}");
    println!("cargo:rustc-env=SUPPORTED_PROTOCOLS={protocols}");
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
