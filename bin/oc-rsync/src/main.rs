// bin/oc-rsync/src/main.rs
use logging::LogFormat;
use std::{io::ErrorKind, path::PathBuf};

use oc_rsync_cli::{cli_command, parse_logging_flags, version_string, EngineError};
use protocol::ExitCode;

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        if !std::env::args().any(|a| a == "--quiet" || a == "-q") {
            print!("{}", version_string());
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
    let (mut info, mut debug) = parse_logging_flags(&matches);
    if quiet {
        info.clear();
        debug.clear();
    }
    let log_format = *matches
        .get_one::<LogFormat>("log_format")
        .unwrap_or(&LogFormat::Text);
    let log_file = matches.get_one::<PathBuf>("client-log-file").cloned();
    let log_file_fmt = matches.get_one::<String>("client-log-file-format").cloned();
    logging::init(
        log_format,
        verbose,
        &info,
        &debug,
        quiet,
        log_file.map(|p| (p, log_file_fmt)),
    );
    if let Err(e) = oc_rsync_cli::run(&matches) {
        eprintln!("{e}");
        let code = match &e {
            EngineError::Io(err)
                if matches!(
                    err.kind(),
                    ErrorKind::TimedOut
                        | ErrorKind::ConnectionRefused
                        | ErrorKind::AddrNotAvailable
                        | ErrorKind::NetworkUnreachable
                        | ErrorKind::WouldBlock
                ) =>
            {
                ExitCode::ConnTimeout
            }
            EngineError::MaxAlloc => ExitCode::Malloc,
            EngineError::Exit(code, _) => *code,
            _ => ExitCode::Protocol,
        };
        std::process::exit(u8::from(code) as i32);
    }
}
