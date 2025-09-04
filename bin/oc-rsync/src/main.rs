// bin/oc-rsync/src/main.rs
use oc_rsync_cli::options::OutBuf;
use oc_rsync_cli::{cli_command, EngineError};
use protocol::ExitCode;
use std::io::ErrorKind;
use std::ptr::{self, NonNull};

extern "C" {
    #[cfg_attr(target_os = "macos", link_name = "__stdoutp")]
    static mut stdout: *mut libc::FILE;
}

#[doc = r"Returns a handle to the C `stdout` stream.

# Safety

Accessing `stdout` from libc requires `unsafe` because it is a mutable static. The pointer is checked for
null to avoid undefined behavior, and callers must ensure no other code closes or invalidates the stream
while it is in use."]
fn stdout_stream() -> std::io::Result<NonNull<libc::FILE>> {
    unsafe {
        NonNull::new(stdout)
            .ok_or_else(|| std::io::Error::new(ErrorKind::BrokenPipe, "stdout is null"))
    }
}

#[doc = r"Sets the buffering mode for a C `FILE` stream.

# Safety

The caller must ensure that `stream` is a valid and open `FILE` pointer."]
fn set_stream_buffer(stream: *mut libc::FILE, mode: libc::c_int) -> std::io::Result<()> {
    if stream.is_null() {
        return Err(std::io::Error::new(ErrorKind::BrokenPipe, "stream is null"));
    }
    let ret = unsafe { libc::setvbuf(stream, ptr::null_mut(), mode, 0) };
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
    let matches = cmd
        .try_get_matches_from_mut(&args)
        .unwrap_or_else(|e| oc_rsync_cli::handle_clap_error(&cmd, e));
    if let Some(mode) = matches.get_one::<OutBuf>("outbuf") {
        let m = match mode {
            OutBuf::N => libc::_IONBF,
            OutBuf::L => libc::_IOLBF,
            OutBuf::B => libc::_IOFBF,
        };
        let stream = match stdout_stream() {
            Ok(s) => s,
            Err(err) => {
                eprintln!("failed to access stdout: {err}");
                std::process::exit(u8::from(ExitCode::FileIo) as i32);
            }
        };
        if let Err(err) = set_stream_buffer(stream.as_ptr(), m) {
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
    use super::set_stream_buffer;
    use clap::error::ErrorKind::*;
    use oc_rsync_cli::exit_code_from_error_kind;
    use protocol::ExitCode;
    use std::ptr;

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

    #[test]
    fn null_stream_returns_error() {
        assert!(set_stream_buffer(ptr::null_mut(), libc::_IONBF).is_err());
    }
}
