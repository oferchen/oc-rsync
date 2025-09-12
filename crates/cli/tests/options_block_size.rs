// crates/cli/tests/options_block_size.rs

use clap::Parser;
use oc_rsync_core::transfer::SyncOptions;

#[derive(Parser, Debug)]
struct Options {
    #[arg(long = "block-size")]
    block_size: Option<String>,
    #[arg(long = "checksum")]
    checksum: bool,
    #[arg()]
    paths: Vec<String>,
}

fn parse_size(value: &str) -> usize {
    if let Some(num) = value.strip_suffix('k') {
        num.parse::<usize>().unwrap() * 1024
    } else {
        value.parse::<usize>().unwrap()
    }
}

fn build_sync_options(opts: &Options) -> SyncOptions {
    SyncOptions {
        checksum: opts.checksum,
        block_size: opts.block_size.as_ref().map(|s| parse_size(s)).unwrap_or(0),
        ..SyncOptions::default()
    }
}

#[test]
fn options_block_size_and_checksum() {
    let opts = Options::try_parse_from([
        "oc-rsync",
        "--block-size",
        "1k",
        "--checksum",
        "src/",
        "dst",
    ])
    .unwrap();

    let sync_opts = build_sync_options(&opts);

    assert_eq!(sync_opts.block_size, 1024);
    assert!(sync_opts.checksum);
}
