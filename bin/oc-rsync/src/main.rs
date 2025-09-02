// bin/oc-rsync/src/main.rs
mod version;
use logging::LogFormat;
use std::{io::ErrorKind, path::PathBuf};

use oc_rsync_cli::{cli_command, parse_logging_flags, EngineError};
use protocol::ExitCode;

fn exit_code_from_error_kind(kind: clap::error::ErrorKind) -> ExitCode {
    use clap::error::ErrorKind::*;
    match kind {
        UnknownArgument => ExitCode::Unsupported,
        _ => ExitCode::SyntaxOrUsage,
    }
}

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        if !std::env::args().any(|a| a == "--quiet" || a == "-q") {
            print!("{}", version::version_banner());
        }
        return;
    }
    let mut cmd = cli_command();
    let matches = cmd
        .try_get_matches_from_mut(std::env::args_os())
        .unwrap_or_else(|e| {
            use clap::error::ErrorKind;
            match e.kind() {
                ErrorKind::DisplayHelp => {
                    println!("{}", oc_rsync_cli::render_help(&cmd));
                    std::process::exit(0);
                }
                kind => {
                    let first = e.to_string();
                    let first = first.lines().next().unwrap_or("");
                    let msg = match kind {
                        ErrorKind::UnknownArgument => {
                            let arg = first.split('\'').nth(1).unwrap_or("");
                            format!("{arg}: unknown option")
                        }
                        _ => first.strip_prefix("error: ").unwrap_or(first).to_string(),
                    };
                    let code = exit_code_from_error_kind(kind);
                    let desc = match code {
                        ExitCode::Unsupported => "requested action not supported",
                        _ => "syntax or usage error",
                    };
                    let code_num = u8::from(code);
                    eprintln!("rsync: {msg}");
                    eprintln!("rsync error: {desc} (code {code_num})");
                    std::process::exit(code_num as i32);
                }
            }
        });
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
    let log_syslog = matches.get_flag("syslog");
    let log_journald = matches.get_flag("journald");
    logging::init(
        log_format,
        verbose,
        &info,
        &debug,
        quiet,
        log_file.map(|p| (p, log_file_fmt)),
        log_syslog,
        log_journald,
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
