// bin/oc-rsync/src/main.rs
mod stdio;

use oc_rsync_cli::options::OutBuf;
use oc_rsync_cli::{cli_command, EngineError};
use protocol::ExitCode;
use std::io::ErrorKind;

fn main() {
    let args: Vec<_> = std::env::args_os().collect();
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
        assert_eq!(
            exit_code_from_error_kind(UnknownArgument),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(InvalidSubcommand),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(exit_code_from_error_kind(NoEquals), ExitCode::SyntaxOrUsage);
        assert_eq!(
            exit_code_from_error_kind(ValueValidation),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(TooManyValues),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(TooFewValues),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(WrongNumberOfValues),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(ArgumentConflict),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(MissingRequiredArgument),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(MissingSubcommand),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(InvalidUtf8),
            ExitCode::SyntaxOrUsage
        );
        assert_eq!(
            exit_code_from_error_kind(DisplayHelpOnMissingArgumentOrSubcommand),
            ExitCode::SyntaxOrUsage,
        );
        assert_eq!(
            exit_code_from_error_kind(InvalidValue),
            ExitCode::Unsupported
        );
        assert_eq!(exit_code_from_error_kind(DisplayHelp), ExitCode::Ok);
        assert_eq!(exit_code_from_error_kind(DisplayVersion), ExitCode::Ok);
        assert_eq!(exit_code_from_error_kind(Io), ExitCode::FileIo);
        assert_eq!(exit_code_from_error_kind(Format), ExitCode::FileIo);
    }
}
