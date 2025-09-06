// crates/cli/tests/logging_flags.rs
use clap::ValueEnum;
use logging::SubscriberConfig;
use oc_rsync_cli::{cli_command, parse_logging_flags};
use tracing::subscriber::with_default;
use tracing::Level;

fn make_sub(
    format: logging::LogFormat,
    verbose: u8,
    info: Vec<logging::InfoFlag>,
    debug: Vec<logging::DebugFlag>,
    quiet: bool,
) -> Box<dyn tracing::Subscriber + Send + Sync> {
    let cfg = SubscriberConfig::builder()
        .format(format)
        .verbose(verbose)
        .info(info)
        .debug(debug)
        .quiet(quiet)
        .log_file(None)
        .colored(true)
        .timestamps(false)
        .build();
    logging::subscriber(cfg).unwrap()
}

#[test]
fn info_flag_enables_progress() {
    let matches = cli_command()
        .try_get_matches_from(["oc-rsync", "--info=progress", "src", "dst"])
        .unwrap();
    let (info, _) = parse_logging_flags(&matches);
    let sub = make_sub(logging::LogFormat::Text, 0, info, vec![], false);
    with_default(sub, || {
        assert!(!tracing::enabled!(Level::INFO));
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
fn info_flag_enables_progress2() {
    let matches = cli_command()
        .try_get_matches_from(["oc-rsync", "--info=progress2", "src", "dst"])
        .unwrap();
    let (info, _) = parse_logging_flags(&matches);
    assert!(info.contains(&logging::InfoFlag::Progress2));
    let sub = make_sub(logging::LogFormat::Text, 0, info, vec![], false);
    with_default(sub, || {
        assert!(tracing::enabled!(
            target: logging::InfoFlag::Progress.target(),
            Level::INFO
        ));
    });
}

#[test]
fn info_flag_enables_stats3() {
    let matches = cli_command()
        .try_get_matches_from(["oc-rsync", "--info=stats3", "src", "dst"])
        .unwrap();
    let (info, _) = parse_logging_flags(&matches);
    assert!(info.contains(&logging::InfoFlag::Stats3));
    let sub = make_sub(logging::LogFormat::Text, 0, info, vec![], false);
    with_default(sub, || {
        assert!(tracing::enabled!(
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
    let (_, debug) = parse_logging_flags(&matches);
    let sub = make_sub(logging::LogFormat::Text, 0, vec![], debug, false);
    with_default(sub, || {
        assert!(!tracing::enabled!(Level::TRACE));
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
fn all_debug_flags_parse() {
    for flag in logging::DebugFlag::value_variants() {
        cli_command()
            .try_get_matches_from([
                "oc-rsync",
                &format!("--debug={}", flag.as_str()),
                "src",
                "dst",
            ])
            .unwrap();
    }
}

#[test]
fn all_info_flags_parse() {
    for flag in logging::InfoFlag::value_variants() {
        cli_command()
            .try_get_matches_from([
                "oc-rsync",
                &format!("--info={}", flag.as_str()),
                "src",
                "dst",
            ])
            .unwrap();
    }
}

#[test]
fn verbose_levels_map_to_tracing() {
    let sub = make_sub(logging::LogFormat::Text, 1, vec![], vec![], false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::INFO));
        assert!(!tracing::enabled!(Level::DEBUG));
    });
    let sub = make_sub(logging::LogFormat::Text, 2, vec![], vec![], false);
    with_default(sub, || {
        assert!(tracing::enabled!(Level::DEBUG));
        assert!(!tracing::enabled!(Level::TRACE));
    });
    let sub = make_sub(logging::LogFormat::Text, 3, vec![], vec![], false);
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
    let (info, debug) = parse_logging_flags(&matches);
    let sub = make_sub(logging::LogFormat::Text, 0, info, debug, true);
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
