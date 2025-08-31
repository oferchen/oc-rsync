// crates/checksums/src/lib.rs
use blake2::{Blake2b512, Blake2s256};
#[cfg(feature = "blake3")]
use blake3::Hasher as Blake3;
use md4::Md4;
use md5::Digest;
use md5::Md5;
use sha1::Sha1;
use xxhash_rust::xxh3::xxh3_128;
use xxhash_rust::xxh64::xxh64;

cpufeatures::new!(sse42, "sse4.2");
cpufeatures::new!(avx2, "avx2");
cpufeatures::new!(avx512, "avx512f");

#[derive(Clone, Copy, Debug)]
pub enum ModernHash {
    #[cfg(feature = "blake3")]
    Blake3,
}

#[derive(Clone, Copy, Debug)]
pub enum StrongHash {
    Md5,
    Sha1,
    Md4,
    Blake2b,
    Blake2s,
    Xxh64,
    Xxh128,
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

#[allow(clippy::needless_borrows_for_generic_args)]
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
        StrongHash::Md4 => {
            let mut hasher = Md4::new();
            hasher.update(&seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::Blake2b => {
            let mut hasher = Blake2b512::new();
            hasher.update(&seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::Blake2s => {
            let mut hasher = Blake2s256::new();
            hasher.update(&seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::Xxh64 => {
            let mut buf = Vec::with_capacity(4 + data.len());
            buf.extend_from_slice(&seed.to_le_bytes());
            buf.extend_from_slice(data);
            xxh64(&buf, 0).to_le_bytes().to_vec()
        }
        StrongHash::Xxh128 => {
            let mut buf = Vec::with_capacity(4 + data.len());
            buf.extend_from_slice(&seed.to_le_bytes());
            buf.extend_from_slice(data);
            xxh3_128(&buf).to_le_bytes().to_vec()
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
    if avx512::get() {
        unsafe { rolling_checksum_avx512(data, seed) }
    } else if avx2::get() {
        unsafe { rolling_checksum_avx2(data, seed) }
    } else if sse42::get() {
        unsafe { rolling_checksum_sse42(data, seed) }
    } else {
        rolling_checksum_scalar(data, seed)
    }
}

#[inline]
fn rolling_checksum_scalar(data: &[u8], seed: u32) -> u32 {
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

#[target_feature(enable = "sse4.2")]
unsafe fn rolling_checksum_sse42(data: &[u8], seed: u32) -> u32 {
    rolling_checksum_scalar(data, seed)
}

#[target_feature(enable = "avx2")]
unsafe fn rolling_checksum_avx2(data: &[u8], seed: u32) -> u32 {
    rolling_checksum_scalar(data, seed)
}

#[target_feature(enable = "avx512f")]
unsafe fn rolling_checksum_avx512(data: &[u8], seed: u32) -> u32 {
    rolling_checksum_scalar(data, seed)
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
        assert_eq!(hex::encode(digest_md5), "be4b47980f89d075f8f7e7a9fab84e29",);

        let digest_sha1 = strong_digest(b"hello world", StrongHash::Sha1, 0);
        assert_eq!(
            hex::encode(digest_sha1),
            "1fb6475c524899f98b088f7608bdab8f1591e078"
        );

        let digest_md4 = strong_digest(b"hello world", StrongHash::Md4, 0);
        assert_eq!(hex::encode(digest_md4), "ea91f391e02b5e19f432b43bd87a531d",);

        let digest_blake2b = strong_digest(b"hello world", StrongHash::Blake2b, 0);
        assert_eq!(
            hex::encode(digest_blake2b),
            "d32b7e7c9028b6e0b1ddd7e83799a8b857a0afcaa370985dfaa42dfa59e275097eb75b99e05bb7ef3ac5cf74c957c3b7cad1dfcbb5e3380d56b63780394af8bd",
        );

        let digest_blake2s = strong_digest(b"hello world", StrongHash::Blake2s, 0);
        assert_eq!(
            hex::encode(digest_blake2s),
            "a2dc531d6048af9ab7cf85108ebcf147632fce6290fbdfcd5ea789a0b31784d0",
        );

        let digest_xxh64 = strong_digest(b"hello world", StrongHash::Xxh64, 0);
        assert_eq!(hex::encode(digest_xxh64), "648e94e9d09503e7");

        let digest_xxh128 = strong_digest(b"hello world", StrongHash::Xxh128, 0);
        assert_eq!(
            hex::encode(digest_xxh128),
            "052acb3009ceb7609305f939f85080da",
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
