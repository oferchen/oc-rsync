// build.rs

use std::{env, fs, path::Path, path::PathBuf, process::Command};

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

    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_ACL");
    if cfg!(unix) {
        if env::var_os("CARGO_FEATURE_ACL").is_some() {
            match pkg_config::Config::new().probe("acl") {
                Ok(_) => {}
                Err(_) => {
                    let mut lib_dir: Option<PathBuf> = None;

                    if let Ok(output) = Command::new("ldconfig").arg("-p").output() {
                        if output.status.success() {
                            let stdout = String::from_utf8_lossy(&output.stdout);
                            for line in stdout.lines() {
                                if line.contains("libacl.so") {
                                    if let Some(path) = line.split("=>").nth(1) {
                                        let path = path.trim();
                                        if let Some(dir) = Path::new(path).parent() {
                                            lib_dir = Some(dir.to_path_buf());
                                            break;
                                        }
                                    }
                                }
                            }
                        }
                    }

                    if lib_dir.is_none() {
                        if let Ok(output) = Command::new("find")
                            .args([
                                "/usr/lib",
                                "/usr/lib64",
                                "/usr/local/lib",
                                "/usr/local/lib64",
                                "/lib",
                                "/lib64",
                            ])
                            .arg("-name")
                            .arg("libacl.so")
                            .arg("-print")
                            .arg("-quit")
                            .output()
                        {
                            if output.status.success() {
                                let stdout = String::from_utf8_lossy(&output.stdout);
                                if let Some(path) = stdout.lines().next() {
                                    if let Some(dir) = Path::new(path.trim()).parent() {
                                        lib_dir = Some(dir.to_path_buf());
                                    }
                                }
                            }
                        }
                    }

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
