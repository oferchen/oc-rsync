// bin/oc-rsync/src/main.rs
use engine::Result;
use logging::LogFormat;
use oc_rsync_cli::cli_command;

fn main() -> Result<()> {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("oc-rsync {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    let matches = cli_command().get_matches();
    let verbose = matches.get_count("verbose") as u8;
    let info = false;
    let debug = false;
    let log_format = matches
        .get_one::<String>("log_format")
        .map(|f| {
            if f == "json" {
                LogFormat::Json
            } else {
                LogFormat::Text
            }
        })
        .unwrap_or(LogFormat::Text);
    logging::init(log_format, verbose, info, debug);
    oc_rsync_cli::run(&matches)
}
