use md5::{Digest, Md5};
use sha1::Sha1;
#[cfg(feature = "blake3")]
use blake3::Hasher as Blake3;

/// Algorithms that can be used for the strong digest.
#[derive(Clone, Copy, Debug)]
pub enum StrongHash {
    Md5,
    Sha1,
    #[cfg(feature = "blake3")]
    Blake3,
}

/// Configuration for checksum computation.
#[derive(Clone, Debug)]
pub struct ChecksumConfig {
    strong: StrongHash,
}

/// Builder for [`ChecksumConfig`].
#[derive(Clone, Debug)]
pub struct ChecksumConfigBuilder {
    strong: StrongHash,
}

impl Default for ChecksumConfigBuilder {
    fn default() -> Self {
        Self {
            strong: StrongHash::Md5,
        }
    }
}

impl ChecksumConfigBuilder {
    /// Create a new builder with default settings (MD5 strong digest).
    pub fn new() -> Self {
        Self::default()
    }

    /// Choose the strong hash algorithm to use when building the config.
    pub fn strong(mut self, alg: StrongHash) -> Self {
        self.strong = alg;
        self
    }

    /// Finalize the builder and produce a [`ChecksumConfig`].
    pub fn build(self) -> ChecksumConfig {
        ChecksumConfig {
            strong: self.strong,
        }
    }
}

/// Result of computing both the rolling checksum and a strong digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksums {
    pub weak: u32,
    pub strong: Vec<u8>,
}

impl ChecksumConfig {
    /// Compute both the rolling checksum and strong digest for `data`.
    pub fn checksum(&self, data: &[u8]) -> Checksums {
        Checksums {
            weak: rolling_checksum(data),
            strong: strong_digest(data, self.strong),
        }
    }
}

/// Compute a strong digest of the data using the requested algorithm.
pub fn strong_digest(data: &[u8], alg: StrongHash) -> Vec<u8> {
    match alg {
        StrongHash::Md5 => {
            let mut hasher = Md5::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        #[cfg(feature = "blake3")]
        StrongHash::Blake3 => {
            let mut hasher = Blake3::new();
            hasher.update(data);
            hasher.finalize().as_bytes().to_vec()
        }
    }
}

/// Compute the rsync rolling checksum for a block of data.
pub fn rolling_checksum(data: &[u8]) -> u32 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    let n = data.len();
    for (i, b) in data.iter().enumerate() {
        s1 = s1.wrapping_add(*b as u32);
        s2 = s2.wrapping_add((n - i) as u32 * (*b as u32));
    }
    (s1 & 0xffff) | (s2 << 16)
}

/// Rolling checksum state allowing incremental updates.
#[derive(Debug, Clone)]
pub struct Rolling {
    len: usize,
    s1: u32,
    s2: u32,
}

impl Rolling {
    pub fn new(block: &[u8]) -> Self {
        let mut r = Rolling {
            len: block.len(),
            s1: 0,
            s2: 0,
        };
        for (i, b) in block.iter().enumerate() {
            r.s1 = r.s1.wrapping_add(*b as u32);
            r.s2 = r.s2.wrapping_add((block.len() - i) as u32 * (*b as u32));
        }
        r
    }

    pub fn roll(&mut self, out: u8, inp: u8) {
        self.s1 = self.s1.wrapping_sub(out as u32).wrapping_add(inp as u32);
        self.s2 = self
            .s2
            .wrapping_sub(self.len as u32 * out as u32)
            .wrapping_add(self.s1);
    }

    pub fn digest(&self) -> u32 {
        (self.s1 & 0xffff) | (self.s2 << 16)
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_known() {
        let sum = rolling_checksum(b"hello world");
        assert_eq!(sum, 436208732); // verified against rsync implementation
    }

    #[test]
    fn rolling_slide() {
        let mut r = Rolling::new(b"hello w");
        r.roll(b'h', b'!');
        assert_eq!(r.digest(), rolling_checksum(b"ello w!"));
    }

    #[test]
    fn strong_digests() {
        let digest_md5 = strong_digest(b"hello world", StrongHash::Md5);
        assert_eq!(hex::encode(digest_md5), "5eb63bbbe01eeed093cb22bb8f5acdc3");

        let digest_sha1 = strong_digest(b"hello world", StrongHash::Sha1);
        assert_eq!(
            hex::encode(digest_sha1),
            "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed"
        );

        #[cfg(feature = "blake3")]
        {
            let digest_blake3 = strong_digest(b"hello world", StrongHash::Blake3);
            assert_eq!(
                hex::encode(digest_blake3),
                "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24"
            );
        }
    }
}
