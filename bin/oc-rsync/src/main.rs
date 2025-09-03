// bin/oc-rsync/src/main.rs
use oc_rsync_cli::options::OutBuf;
use oc_rsync_cli::{branding, cli_command, EngineError};
use protocol::ExitCode;
use std::io::ErrorKind;
use std::ptr;

extern "C" {
    static mut stdout: *mut libc::FILE;
}

fn exit_code_from_error_kind(kind: clap::error::ErrorKind) -> ExitCode {
    use clap::error::ErrorKind::*;
    match kind {
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
        DisplayHelpOnMissingArgumentOrSubcommand => ExitCode::SyntaxOrUsage,
        InvalidValue => ExitCode::Unsupported,
        DisplayHelp => ExitCode::Ok,
        DisplayVersion => ExitCode::Ok,
        Io => ExitCode::FileIo,
        Format => ExitCode::FileIo,
        _ => ExitCode::SyntaxOrUsage,
    }
}

unsafe fn set_stream_buffer(stream: *mut libc::FILE, mode: libc::c_int) -> std::io::Result<()> {
    let ret = libc::setvbuf(stream, ptr::null_mut(), mode, 0);
    if ret == 0 {
        Ok(())
    } else {
        Err(std::io::Error::last_os_error())
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
            let prog = branding::program_name();
            eprintln!("{prog}: {msg}");
            eprintln!("{prog} error: {desc} (code {code_num})");
        }
        std::process::exit(u8::from(code) as i32);
    });
    if let Some(mode) = matches.get_one::<OutBuf>("outbuf") {
        unsafe {
            let m = match mode {
                OutBuf::N => libc::_IONBF,
                OutBuf::L => libc::_IOLBF,
                OutBuf::B => libc::_IOFBF,
            };
            if let Err(err) = set_stream_buffer(stdout, m) {
                eprintln!("failed to set stdout buffer: {err}");
                std::process::exit(u8::from(ExitCode::FileIo) as i32);
            }
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

#[cfg(test)]
mod tests {
    use super::exit_code_from_error_kind;
    use super::set_stream_buffer;
    use clap::error::ErrorKind::*;
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

    #[test]
    fn invalid_setvbuf_returns_error() {
        unsafe {
            let file = libc::tmpfile();
            assert!(!file.is_null());
            assert!(set_stream_buffer(file, -1).is_err());
            libc::fclose(file);
        }
    }
}
