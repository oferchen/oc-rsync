// crates/engine/src/xattrs.rs
use crate::{EngineError, Result};

pub fn ensure_supported() -> Result<()> {
    #[cfg(feature = "xattr")]
    {
        if meta::xattrs_supported() {
            Ok(())
        } else {
            Err(EngineError::Other(
                "filesystem does not support extended attributes".into(),
            ))
        }
    }
    #[cfg(not(feature = "xattr"))]
    {
        Err(EngineError::Other(
            "binary was built without xattr support".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "xattr")]
    #[test]
    fn supported_fs_returns_ok() {
        if meta::xattrs_supported() {
            ensure_supported().unwrap();
        } else {
            println!("skipping: xattrs unsupported");
        }
    }

    #[cfg(feature = "xattr")]
    #[test]
    fn unsupported_fs_returns_err() {
        if meta::xattrs_supported() {
            println!("skipping: xattrs supported");
        } else {
            assert!(ensure_supported().is_err());
        }
    }

    #[cfg(not(feature = "xattr"))]
    #[test]
    fn feature_disabled_returns_err() {
        assert!(ensure_supported().is_err());
    }
}
