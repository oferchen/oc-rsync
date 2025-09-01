// bin/oc-rsync/src/main.rs
use logging::{DebugFlag, InfoFlag, LogFormat};

use oc_rsync_cli::{cli_command, EngineError};
use protocol::ExitCode;

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        if !std::env::args().any(|a| a == "--quiet" || a == "-q") {
            println!("oc-rsync {}", env!("CARGO_PKG_VERSION"));
        }
        return;
    }
    let matches = cli_command().get_matches();
    let quiet = matches.get_flag("quiet");
    let verbose = if quiet {
        0
    } else {
        matches.get_count("verbose") as u8
    };
    let info: Vec<InfoFlag> = if quiet {
        Vec::new()
    } else {
        matches
            .get_many::<InfoFlag>("info")
            .map(|v| v.copied().collect())
            .unwrap_or_default()
    };
    let debug: Vec<DebugFlag> = if quiet {
        Vec::new()
    } else {
        matches
            .get_many::<DebugFlag>("debug")
            .map(|v| v.copied().collect())
            .unwrap_or_default()
    };
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
    logging::init(log_format, verbose, &info, &debug, quiet);
    if let Err(e) = oc_rsync_cli::run(&matches) {
        eprintln!("{e}");
        let code = match e {
            EngineError::MaxAlloc => ExitCode::Malloc,
            _ => ExitCode::Protocol,
        };
        std::process::exit(u8::from(code) as i32);
    }
}
