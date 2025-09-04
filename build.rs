const UPSTREAM_VERSION: &str = "3.4.1";
const UPSTREAM_PROTOCOLS: &[u32] = &[32, 31, 30, 29];

fn main() {
    let protocols = UPSTREAM_PROTOCOLS
        .iter()
        .map(|p| p.to_string())
        .collect::<Vec<_>>()
        .join(",");

    println!("cargo:rustc-env=UPSTREAM_VERSION={UPSTREAM_VERSION}");
    println!("cargo:rustc-env=UPSTREAM_PROTOCOLS={protocols}");

    println!("cargo:rerun-if-env-changed=UPSTREAM_VERSION");
    println!("cargo:rerun-if-env-changed=UPSTREAM_PROTOCOLS");
}
