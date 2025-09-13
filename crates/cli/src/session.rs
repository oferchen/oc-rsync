// crates/cli/src/session.rs

use oc_rsync_core::message::CharsetConv;
use oc_rsync_core::transfer::{EngineError, Result};
use transport::SshStdioTransport;

fn escape_bytes(bytes: &[u8]) -> String {
    let mut out = String::new();
    for &b in bytes {
        if (b < 0x20 && b != b'\t') || b > 0x7e {
            out.push_str(&format!("\\#{:03o}", b));
        } else {
            out.push(char::from(b));
        }
    }
    out
}

pub(crate) fn check_session_errors(
    session: &SshStdioTransport,
    iconv: Option<&CharsetConv>,
) -> Result<()> {
    let (err, _) = session.stderr();
    if !err.is_empty() {
        let msg = if let Some(cv) = iconv {
            cv.decode_remote(&err).into_owned()
        } else {
            match String::from_utf8(err) {
                Ok(s) => s,
                Err(e) => escape_bytes(&e.into_bytes()),
            }
        };
        return Err(EngineError::Other(msg));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::time::Duration;

    #[test]
    fn ok_on_empty_stderr() {
        let session = SshStdioTransport::spawn("true", std::iter::empty::<&str>()).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(check_session_errors(&session, None).is_ok());
    }

    #[test]
    fn utf8_error_message() {
        let session = SshStdioTransport::spawn("sh", ["-c", "printf 'err' >&2"]).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        match check_session_errors(&session, None) {
            Err(EngineError::Other(msg)) => assert_eq!(msg, "err"),
            _ => panic!(),
        }
    }

    #[test]
    fn escapes_non_utf8_error() {
        let session = SshStdioTransport::spawn("sh", ["-c", "printf 'f\\377f' >&2"]).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        let expected =
            fs::read_to_string("../../tests/fixtures/rsync-send-nonascii-default.txt").unwrap();
        let expected = expected.trim_end().trim_start_matches("send");
        match check_session_errors(&session, None) {
            Err(EngineError::Other(msg)) => assert_eq!(msg, expected),
            _ => panic!(),
        }
    }
}
