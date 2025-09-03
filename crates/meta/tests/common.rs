// crates/meta/tests/common.rs
use meta::Options;

pub fn full_metadata_opts() -> Options {
    Options {
        owner: true,
        group: true,
        perms: true,
        times: true,
        atimes: true,
        crtimes: true,
        ..Default::default()
    }
}
