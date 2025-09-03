# Failing test report

The test suite currently has numerous failures. Most integration tests invoking the `oc-rsync` binary exit with a usage error indicating that required positional arguments are missing, suggesting the CLI argument parser is incomplete.

## Default features

- **archive**: `archive_matches_combination_and_rsync`, `archive_respects_no_options` — both report `rsync: the following required arguments were not provided` when run with `-a` and source/destination paths, showing the CLI rejects valid invocations.
- **cli**: 93 failures (e.g., `checksum_forces_transfer_cli`, `chmod_masks_file_type_bits`). These all emit the same missing-argument usage error, pointing to broad gaps in CLI support.
- **cli_flags**: `delete_flags_last_one_wins` fails because combining delete flags triggers a `MissingRequiredArgument <DST>` error.
- **copy_as**, **cvs_exclude**, **delete_policy**, **dry_run**, **filter_corpus**, **ignore_missing_args**, **link_copy_compare_dest**, **local_sync_tree**, **log_file**, **modify_window**, **option_errors**, **perf_limits**, **remote_remote**, **rsh**, **rsync_path**, **secluded_args**, **skip_compress**, **sockopts**, **symlink_resolution**, **timeout**, **version_output**, **write_batch**, **write_devices**: all exhibit the same usage error, implying incomplete CLI implementation across many options.
- **daemon** and **daemon_config**: multiple tests fail with usage errors when launching the daemon, indicating incomplete daemon/transport handling.
- **resume**: tests such as `append_resumes_after_interrupt` fail assertions comparing transferred byte counts, showing resume/partial-transfer logic is not yet implemented.

## Feature-gated runs

- `--features xattr`: `archive` tests fail with the same missing-argument usage error.
- `--features acl`: compilation fails because the system lacks `libacl` (`ld: cannot find -lacl`).
- `--features cli`: `archive` tests continue to report missing arguments.
- `--features nightly`: `block_size` tests pass.

## Other checks

- `make verify-comments` reports `crates/transport/tests/reject.rs: additional comments`.
- `make lint` (cargo fmt --all --check) passes.
- `cargo clippy --all-targets --all-features -- -D warnings` passes.

No relevant upstream issues or pull requests were found.
