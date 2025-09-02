// bin/oc-rsync/src/main.rs
use oc_rsync_cli::{cli_command, EngineError};
use protocol::ExitCode;
use std::io::ErrorKind;

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
        | DisplayHelpOnMissingArgumentOrSubcommand => ExitCode::SyntaxOrUsage,
        DisplayHelp | DisplayVersion => ExitCode::Ok,
        Io | Format => ExitCode::FileIo,
        _ => ExitCode::SyntaxOrUsage,
    }
}

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if oc_rsync_cli::print_version_if_requested(args.iter().cloned()) {
        return;
    }
    let mut cmd = cli_command();
    let matches = cmd.try_get_matches_from_mut(&args).unwrap_or_else(|e| {
        use clap::error::ErrorKind;
        let kind = e.kind();
        let code = exit_code_from_error_kind(kind);
        if kind == ErrorKind::DisplayHelp {
            println!("{}", oc_rsync_cli::render_help(&cmd));
        } else {
            let first = e.to_string();
            let first = first.lines().next().unwrap_or("");
            let msg = match kind {
                ErrorKind::UnknownArgument => {
                    let arg = first.split('\'').nth(1).unwrap_or("");
                    format!("{arg}: unknown option")
                }
                _ => first.strip_prefix("error: ").unwrap_or(first).to_string(),
            };
            let desc = match code {
                ExitCode::Unsupported => "requested action not supported",
                _ => "syntax or usage error",
            };
            let code_num = u8::from(code);
            eprintln!("rsync: {msg}");
            eprintln!("rsync error: {desc} (code {code_num})");
        }
        std::process::exit(u8::from(code) as i32);
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
