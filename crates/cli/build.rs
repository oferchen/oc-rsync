use std::env;
use time::OffsetDateTime;

fn main() {
    let year =
        env::var("CURRENT_YEAR").unwrap_or_else(|_| OffsetDateTime::now_utc().year().to_string());
    println!("cargo:rustc-env=CURRENT_YEAR={year}");
    println!("cargo:rerun-if-env-changed=CURRENT_YEAR");
}
