// crates/cli/src/session.rs

use oc_rsync_core::message::CharsetConv;
use oc_rsync_core::transfer::{EngineError, Result};
use transport::SshStdioTransport;

pub(crate) fn check_session_errors(
    session: &SshStdioTransport,
    iconv: Option<&CharsetConv>,
) -> Result<()> {
    let (err, _) = session.stderr();
    if !err.is_empty() {
        let msg = if let Some(cv) = iconv {
            cv.decode_remote(&err).into_owned()
        } else {
            String::from_utf8_lossy(&err).into_owned()
        };
        return Err(EngineError::Other(msg));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn detects_session_error() {
        let session = SshStdioTransport::spawn("sh", ["-c", "echo err >&2"]).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(check_session_errors(&session, None).is_err());
    }

    #[test]
    fn ok_on_empty_stderr() {
        let session = SshStdioTransport::spawn("true", std::iter::empty::<&str>()).unwrap();
        std::thread::sleep(Duration::from_millis(50));
        assert!(check_session_errors(&session, None).is_ok());
    }
}
