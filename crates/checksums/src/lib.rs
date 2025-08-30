// crates/checksums/src/lib.rs
#[cfg(feature = "blake3")]
use blake3::Hasher as Blake3;
use md5::{Digest, Md5};
use sha1::Sha1;

#[derive(Clone, Copy, Debug)]
pub enum StrongHash {
    Md5,
    Sha1,
    #[cfg(feature = "blake3")]
    Blake3,
}

#[derive(Clone, Debug)]
pub struct ChecksumConfig {
    strong: StrongHash,
    seed: u32,
}

#[derive(Clone, Debug)]
pub struct ChecksumConfigBuilder {
    strong: StrongHash,
    seed: u32,
}

impl Default for ChecksumConfigBuilder {
    fn default() -> Self {
        Self {
            strong: StrongHash::Md5,
            seed: 0,
        }
    }
}

impl ChecksumConfigBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn strong(mut self, alg: StrongHash) -> Self {
        self.strong = alg;
        self
    }

    pub fn seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    pub fn build(self) -> ChecksumConfig {
        ChecksumConfig {
            strong: self.strong,
            seed: self.seed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Checksums {
    pub weak: u32,
    pub strong: Vec<u8>,
}

impl ChecksumConfig {
    pub fn checksum(&self, data: &[u8]) -> Checksums {
        Checksums {
            weak: rolling_checksum_seeded(data, self.seed),
            strong: strong_digest(data, self.strong, self.seed),
        }
    }
}

pub fn strong_digest(data: &[u8], alg: StrongHash, seed: u32) -> Vec<u8> {
    match alg {
        StrongHash::Md5 => {
            let mut hasher = Md5::new();
            hasher.update(&seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(&seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        #[cfg(feature = "blake3")]
        StrongHash::Blake3 => {
            let mut hasher = Blake3::new();
            hasher.update(&seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().as_bytes().to_vec()
        }
    }
}

pub fn rolling_checksum(data: &[u8]) -> u32 {
    rolling_checksum_seeded(data, 0)
}

pub fn rolling_checksum_seeded(data: &[u8], seed: u32) -> u32 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    let n = data.len();
    for (i, b) in data.iter().enumerate() {
        s1 = s1.wrapping_add(*b as u32);
        s2 = s2.wrapping_add((n - i) as u32 * (*b as u32));
    }
    s1 = s1.wrapping_add(seed);
    s2 = s2.wrapping_add((n as u32).wrapping_mul(seed));
    (s1 & 0xffff) | (s2 << 16)
}

#[derive(Debug, Clone)]
pub struct Rolling {
    len: usize,
    s1: u32,
    s2: u32,
    seed: u32,
}

impl Rolling {
    pub fn new(block: &[u8]) -> Self {
        Self::with_seed(block, 0)
    }

    pub fn with_seed(block: &[u8], seed: u32) -> Self {
        let mut r = Rolling {
            len: block.len(),
            s1: 0,
            s2: 0,
            seed,
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
        let s1 = self.s1.wrapping_add(self.seed);
        let s2 = self
            .s2
            .wrapping_add((self.len as u32).wrapping_mul(self.seed));
        (s1 & 0xffff) | (s2 << 16)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_known() {
        let sum = rolling_checksum(b"hello world");
        assert_eq!(sum, 436208732);
    }

    #[test]
    fn rolling_slide() {
        let mut r = Rolling::new(b"hello w");
        r.roll(b'h', b'!');
        assert_eq!(r.digest(), rolling_checksum(b"ello w!"));
    }

    #[test]
    fn strong_digests() {
        let digest_md5 = strong_digest(b"hello world", StrongHash::Md5, 0);
        assert_eq!(hex::encode(digest_md5), "be4b47980f89d075f8f7e7a9fab84e29");

        let digest_sha1 = strong_digest(b"hello world", StrongHash::Sha1, 0);
        assert_eq!(
            hex::encode(digest_sha1),
            "1fb6475c524899f98b088f7608bdab8f1591e078"
        );

        #[cfg(feature = "blake3")]
        {
            let digest_blake3 = strong_digest(b"hello world", StrongHash::Blake3, 0);
            assert_eq!(
                hex::encode(digest_blake3),
                "861487254e43e2e567ef5177d0c85452f1982ec89c494e8d4a957ff01dd9b421"
            );
        }
    }
}
