use std::fmt;
use std::io;

use crate::versions::{SUPPORTED_CAPS, SUPPORTED_PROTOCOLS};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VersionError(pub u32);

impl fmt::Display for VersionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported version {}", self.0)
    }
}

impl std::error::Error for VersionError {}

impl From<VersionError> for io::Error {
    fn from(e: VersionError) -> Self {
        io::Error::new(io::ErrorKind::InvalidData, e)
    }
}

pub fn negotiate_version(local: u32, peer: u32) -> Result<u32, VersionError> {
    for &v in SUPPORTED_PROTOCOLS {
        if local >= v && peer >= v {
            return Ok(v);
        }
    }
    Err(VersionError(local.min(peer)))
}

pub fn negotiate_caps(local: u32, peer: u32) -> u32 {
    (local & peer) & SUPPORTED_CAPS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::versions::{MIN_VERSION, SUPPORTED_PROTOCOLS};

    #[test]
    fn version_negotiation() {
        let latest = SUPPORTED_PROTOCOLS[0];
        for &peer in SUPPORTED_PROTOCOLS {
            assert_eq!(negotiate_version(latest, peer), Ok(peer));
            assert_eq!(negotiate_version(peer, latest), Ok(peer));
        }
        assert!(negotiate_version(latest, MIN_VERSION - 1).is_err());
    }
}
