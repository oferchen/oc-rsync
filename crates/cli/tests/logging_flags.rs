use assert_cmd::Command;
use oc_rsync_cli::cli_command;
use tempfile::tempdir;

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
    let info = false;
    let debug = false;
    let log_format = if matches.get_one::<String>("log_format").map(String::as_str) == Some("json")
    {
        logging::LogFormat::Json
    } else {
        logging::LogFormat::Text
    };
    logging::init(log_format, verbose, info, debug);
    oc_rsync_cli::run(&matches).unwrap();
}
