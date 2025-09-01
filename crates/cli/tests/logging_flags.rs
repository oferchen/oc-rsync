// crates/cli/tests/logging_flags.rs
use assert_cmd::Command;
use clap::ValueEnum;
use oc_rsync_cli::cli_command;
use tempfile::tempdir;
use tracing::subscriber::with_default;
use tracing::Level;

#[test]
fn verbose_and_log_format_json_parity() {
    let src = tempdir().unwrap();
    let dst = tempdir().unwrap();
    let src_path = src.path();
    let dst_path = dst.path();

    Command::cargo_bin("oc-rsync")
        .unwrap()
        .args([
            "--verbose",
            "--log-format=json",
            "--dry-run",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "--verbose",
            "--log-format=json",
            "--dry-run",
            src_path.to_str().unwrap(),
            dst_path.to_str().unwrap(),
        ])
        .unwrap();
    let verbose = matches.get_count("verbose") as u8;
    let info: Vec<logging::InfoFlag> = matches
        .get_many::<logging::InfoFlag>("info")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    let debug: Vec<logging::DebugFlag> = matches
        .get_many::<logging::DebugFlag>("debug")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    let log_format = *matches
        .get_one::<logging::LogFormat>("log_format")
        .unwrap_or(&logging::LogFormat::Text);
    logging::init(log_format, verbose, &info, &debug, false, None);
    oc_rsync_cli::run(&matches).unwrap();
}

#[test]
fn info_flag_enables_progress() {
    let matches = cli_command()
        .try_get_matches_from(["oc-rsync", "--info=progress", "src", "dst"])
        .unwrap();
    let info: Vec<logging::InfoFlag> = matches
        .get_many::<logging::InfoFlag>("info")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    let sub = logging::subscriber(logging::LogFormat::Text, 0, &info, &[], false, None);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
        assert!(!tracing::enabled!(Level::DEBUG));
        assert!(tracing::enabled!(
            target: logging::InfoFlag::Progress.target(),
            Level::INFO
        ));
        assert!(!tracing::enabled!(
            target: logging::InfoFlag::Stats.target(),
            Level::INFO
        ));
    });
}

#[test]
fn debug_flag_enables_flist() {
    let matches = cli_command()
        .try_get_matches_from(["oc-rsync", "--debug=flist", "src", "dst"])
        .unwrap();
    let debug: Vec<logging::DebugFlag> = matches
        .get_many::<logging::DebugFlag>("debug")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    let sub = logging::subscriber(logging::LogFormat::Text, 0, &[], &debug, false, None);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::TRACE));
        assert!(tracing::enabled!(
            target: logging::DebugFlag::Flist.target(),
            Level::DEBUG
        ));
        assert!(!tracing::enabled!(
            target: logging::DebugFlag::Hash.target(),
            Level::DEBUG
        ));
    });
}

#[test]
fn all_debug_flags_enable_targets() {
    for flag in logging::DebugFlag::value_variants() {
        let matches = cli_command()
            .try_get_matches_from([
                "oc-rsync",
                &format!("--debug={}", flag.as_str()),
                "src",
                "dst",
            ])
            .unwrap();
        let debug: Vec<logging::DebugFlag> = matches
            .get_many::<logging::DebugFlag>("debug")
            .map(|v| v.copied().collect())
            .unwrap_or_default();
        let sub = logging::subscriber(logging::LogFormat::Text, 0, &[], &debug, false, None);
        with_default(sub, || {
            assert!(tracing::enabled!(Level::TRACE));
            assert!(tracing::enabled!(target: flag.target(), Level::DEBUG));
        });
    }
}

#[test]
fn verbose_levels_map_to_tracing() {
    let sub = logging::subscriber(logging::LogFormat::Text, 1, &[], &[], false, None);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
        assert!(!tracing::enabled!(Level::DEBUG));
    });
    let sub = logging::subscriber(logging::LogFormat::Text, 2, &[], &[], false, None);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
        assert!(!tracing::enabled!(Level::TRACE));
    });
    let sub = logging::subscriber(logging::LogFormat::Text, 3, &[], &[], false, None);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::TRACE));
    });
}

#[test]
fn quiet_overrides_logging_flags() {
    let matches = cli_command()
        .try_get_matches_from([
            "oc-rsync",
            "--quiet",
            "--info=progress",
            "--debug=flist",
            "src",
            "dst",
        ])
        .unwrap();
    let info: Vec<logging::InfoFlag> = matches
        .get_many::<logging::InfoFlag>("info")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    let debug: Vec<logging::DebugFlag> = matches
        .get_many::<logging::DebugFlag>("debug")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    let sub = logging::subscriber(logging::LogFormat::Text, 0, &info, &debug, true, None);
    with_default(sub, || {
        assert!(!tracing::enabled!(Level::INFO));
        assert!(!tracing::enabled!(Level::DEBUG));
        assert!(!tracing::enabled!(
            target: logging::InfoFlag::Progress.target(),
            Level::INFO
        ));
        assert!(!tracing::enabled!(
            target: logging::DebugFlag::Flist.target(),
            Level::DEBUG
        ));
    });
}
