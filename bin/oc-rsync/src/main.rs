// bin/oc-rsync/src/main.rs
mod version;

use logging::LogFormat;
use oc_rsync_cli::{cli_command, parse_logging_flags, EngineError};
use protocol::ExitCode;
use std::{io::ErrorKind, path::PathBuf};

fn exit_code_from_error_kind(kind: clap::error::ErrorKind) -> ExitCode {
    use clap::error::ErrorKind::*;
    match kind {
        UnknownArgument => ExitCode::Unsupported,
        InvalidValue
        | InvalidSubcommand
        | NoEquals
        | ValueValidation
        | TooManyValues
        | TooFewValues
        | WrongNumberOfValues
        | ArgumentConflict
        | MissingRequiredArgument
        | MissingSubcommand
        | InvalidUtf8
        | DisplayHelp
        | DisplayHelpOnMissingArgumentOrSubcommand
        | DisplayVersion => ExitCode::SyntaxOrUsage,
        Io | Format => ExitCode::FileIo,
        _ => ExitCode::SyntaxOrUsage,
    }
}

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        if !std::env::args().any(|a| a == "--quiet" || a == "-q") {
            println!("{}", version::render_version_lines().join("\n"));
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
