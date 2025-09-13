// crates/cli/src/argparse/validation.rs
use std::ffi::OsString;

use crate::{EngineError, RemoteSpec, parse_remote_spec};
use oc_rsync_core::message::ExitCode;
use oc_rsync_core::transfer::Result;

use super::ClientOpts;

pub fn validate_paths(opts: &ClientOpts) -> Result<(Vec<OsString>, OsString)> {
    if opts.paths.len() < 2 {
        return Err(EngineError::Other("missing SRC or DST".into()));
    }
    let dst_arg = opts
        .paths
        .last()
        .cloned()
        .ok_or_else(|| EngineError::Other("missing SRC or DST".into()))?;
    let srcs = opts.paths[..opts.paths.len() - 1].to_vec();
    if srcs.len() > 1 {
        if let Ok(RemoteSpec::Local(ps)) = parse_remote_spec(dst_arg.as_os_str()) {
            if ps.path.exists() && !ps.path.is_dir() {
                return Err(EngineError::Other(
                    "destination must be a directory when copying more than 1 file".into(),
                ));
            }
        }
    }
    Ok((srcs, dst_arg))
}

pub fn exit_code_from_error_kind(kind: clap::error::ErrorKind) -> ExitCode {
    use clap::error::ErrorKind::*;
    match kind {
        InvalidValue => ExitCode::Unsupported,
        UnknownArgument => ExitCode::SyntaxOrUsage,
        InvalidSubcommand => ExitCode::SyntaxOrUsage,
        NoEquals => ExitCode::SyntaxOrUsage,
        ValueValidation => ExitCode::SyntaxOrUsage,
        TooManyValues => ExitCode::SyntaxOrUsage,
        TooFewValues => ExitCode::SyntaxOrUsage,
        WrongNumberOfValues => ExitCode::SyntaxOrUsage,
        ArgumentConflict => ExitCode::SyntaxOrUsage,
        MissingRequiredArgument => ExitCode::SyntaxOrUsage,
        MissingSubcommand => ExitCode::SyntaxOrUsage,
        InvalidUtf8 => ExitCode::SyntaxOrUsage,
        DisplayHelp => ExitCode::Ok,
        DisplayHelpOnMissingArgumentOrSubcommand => ExitCode::SyntaxOrUsage,
        DisplayVersion => ExitCode::Ok,
        Io => ExitCode::FileIo,
        Format => ExitCode::FileIo,
        #[allow(unreachable_patterns)]
        _ => ExitCode::SyntaxOrUsage,
    }
}

pub fn exit_code_from_engine_error(e: &EngineError) -> ExitCode {
    use std::io::ErrorKind;
    match e {
        EngineError::Io(err) => match err.kind() {
            ErrorKind::TimedOut | ErrorKind::WouldBlock => ExitCode::ConnTimeout,
            ErrorKind::ConnectionRefused
            | ErrorKind::AddrNotAvailable
            | ErrorKind::NetworkUnreachable
            | ErrorKind::ConnectionAborted
            | ErrorKind::ConnectionReset
            | ErrorKind::NotConnected
            | ErrorKind::HostUnreachable
            | ErrorKind::NetworkDown => ExitCode::SocketIo,
            _ => ExitCode::Protocol,
        },
        EngineError::MaxAlloc => ExitCode::Malloc,
        EngineError::Exit(code, _) => *code,
        _ => ExitCode::Protocol,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_handles_unknown_error_kind() {
        let kind = clap::error::ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand;
        assert_eq!(exit_code_from_error_kind(kind), ExitCode::SyntaxOrUsage);
    }

    #[test]
    fn maps_error_kinds_to_exit_codes() {
        use clap::error::ErrorKind::*;
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

    #[test]
    fn transient_network_errors_map_to_conn_timeout() {
        use std::io::ErrorKind;
        let kinds = [
            ErrorKind::TimedOut,
            ErrorKind::ConnectionRefused,
            ErrorKind::AddrNotAvailable,
            ErrorKind::NetworkUnreachable,
            ErrorKind::WouldBlock,
            ErrorKind::ConnectionAborted,
            ErrorKind::ConnectionReset,
            ErrorKind::NotConnected,
            ErrorKind::HostUnreachable,
            ErrorKind::NetworkDown,
        ];

        for kind in kinds {
            let err = EngineError::Io(std::io::Error::from(kind));
            assert_eq!(
                exit_code_from_engine_error(&err),
                ExitCode::ConnTimeout,
                "{kind:?} did not map to ConnTimeout",
            );
        }
    }
}
