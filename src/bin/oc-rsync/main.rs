// src/bin/oc-rsync/main.rs
mod stdio;

use oc_rsync_cli::options::OutBuf;
use oc_rsync_cli::{cli_command, EngineError};
use protocol::ExitCode;
use std::io::ErrorKind;

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
    if args.iter().any(|a| a == "--dump-help-body") {
        let cmd = cli_command();
        let help = oc_rsync_cli::render_help(&cmd);
        let mut parts = help.splitn(4, '\n');
        parts.next();
        parts.next();
        parts.next();
        if let Some(body) = parts.next() {
            println!("{body}");
        }
        return;
    }
    if oc_rsync_cli::print_version_if_requested(args.iter().cloned()) {
        return;
    }
    let mut cmd = cli_command();
    let matches = cmd
        .try_get_matches_from_mut(&args)
        .unwrap_or_else(|e| oc_rsync_cli::handle_clap_error(&cmd, e));
    if let Some(mode) = matches.get_one::<OutBuf>("outbuf") {
        if let Err(err) = stdio::set_stdout_buffering(*mode) {
            eprintln!("failed to set stdout buffer: {err}");
            std::process::exit(u8::from(ExitCode::FileIo) as i32);
        }
    }
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
                        | ErrorKind::WouldBlock,
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

#[cfg(test)]
mod tests {
    use clap::error::ErrorKind::*;
    use oc_rsync_cli::exit_code_from_error_kind;
    use protocol::ExitCode;

    #[test]
    fn maps_error_kinds_to_exit_codes() {
        let cases = [
            (InvalidValue, ExitCode::Unsupported),
            (UnknownArgument, ExitCode::SyntaxOrUsage),
            (InvalidSubcommand, ExitCode::SyntaxOrUsage),
            (NoEquals, ExitCode::SyntaxOrUsage),
            (ValueValidation, ExitCode::SyntaxOrUsage),
            (TooManyValues, ExitCode::SyntaxOrUsage),
            (TooFewValues, ExitCode::SyntaxOrUsage),
            (WrongNumberOfValues, ExitCode::SyntaxOrUsage),
            (ArgumentConflict, ExitCode::SyntaxOrUsage),
            (MissingRequiredArgument, ExitCode::SyntaxOrUsage),
            (MissingSubcommand, ExitCode::SyntaxOrUsage),
            (InvalidUtf8, ExitCode::SyntaxOrUsage),
            (
                DisplayHelpOnMissingArgumentOrSubcommand,
                ExitCode::SyntaxOrUsage,
            ),
            (DisplayHelp, ExitCode::Ok),
            (DisplayVersion, ExitCode::Ok),
            (Io, ExitCode::FileIo),
            (Format, ExitCode::FileIo),
        ];

        for (kind, expected) in cases {
            assert_eq!(exit_code_from_error_kind(kind), expected);
        }
    }
}
