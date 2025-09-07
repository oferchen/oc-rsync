// crates/cli/tests/options_block_size.rs
use engine::SyncOptions;
use oc_rsync_cli::options::ClientOpts as Options;

fn build_sync_options(opts: &Options) -> SyncOptions {
    SyncOptions {
        checksum: opts.checksum,
        block_size: opts.block_size.unwrap_or(0),
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
