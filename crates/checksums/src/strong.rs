// crates/checksums/src/strong.rs

use md4::{Digest, Md4};
use md5::Md5;
use sha1::Sha1;
use xxhash_rust::xxh64::{Xxh64, xxh64};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StrongHash {
    Md4,
    Md5,
    Sha1,
    XxHash,
}

pub trait StrongChecksum: Send {
    fn update(&mut self, data: &[u8]);
    fn finalize(self: Box<Self>) -> Vec<u8>;
}

struct Md4Checksum {
    hasher: Md4,
    seed: u32,
}

impl StrongChecksum for Md4Checksum {
    fn update(&mut self, data: &[u8]) {
        self.hasher.update(data);
    }

    fn finalize(mut self: Box<Self>) -> Vec<u8> {
        self.hasher.update(self.seed.to_le_bytes());
        self.hasher.finalize().to_vec()
    }
}

struct Md5Checksum(Md5);

impl StrongChecksum for Md5Checksum {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

struct Sha1Checksum(Sha1);

impl StrongChecksum for Sha1Checksum {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        self.0.finalize().to_vec()
    }
}

struct XxHashChecksum(Xxh64);

impl StrongChecksum for XxHashChecksum {
    fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    fn finalize(self: Box<Self>) -> Vec<u8> {
        self.0.digest().to_le_bytes().to_vec()
    }
}

pub fn select_strong_checksum(alg: StrongHash, seed: u32) -> Box<dyn StrongChecksum> {
    match alg {
        StrongHash::Md4 => Box::new(Md4Checksum {
            hasher: Md4::new(),
            seed,
        }),
        StrongHash::Md5 => {
            let mut h = Md5::new();
            h.update(seed.to_le_bytes());
            Box::new(Md5Checksum(h))
        }
        StrongHash::Sha1 => {
            let mut h = Sha1::new();
            h.update(seed.to_le_bytes());
            Box::new(Sha1Checksum(h))
        }
        StrongHash::XxHash => Box::new(XxHashChecksum(Xxh64::new(seed as u64))),
    }
}

#[allow(clippy::needless_borrows_for_generic_args)]
pub fn strong_digest(data: &[u8], alg: StrongHash, seed: u32) -> Vec<u8> {
    match alg {
        StrongHash::Md4 => {
            let mut hasher = Md4::new();
            hasher.update(data);
            hasher.update(seed.to_le_bytes());
            hasher.finalize().to_vec()
        }
        StrongHash::Md5 => {
            let mut hasher = Md5::new();
            hasher.update(seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::Sha1 => {
            let mut hasher = Sha1::new();
            hasher.update(seed.to_le_bytes());
            hasher.update(data);
            hasher.finalize().to_vec()
        }
        StrongHash::XxHash => xxh64(data, seed as u64).to_le_bytes().to_vec(),
    }
}

pub fn available_strong_hashes() -> &'static [StrongHash] {
    &[
        StrongHash::XxHash,
        StrongHash::Md5,
        StrongHash::Md4,
        StrongHash::Sha1,
    ]
}

pub fn negotiate_strong_hash(local: &[StrongHash], remote: &[StrongHash]) -> Option<StrongHash> {
    local.iter().copied().find(|h| remote.contains(h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strong_digests() {
        let digest_md4 = strong_digest(b"hello world", StrongHash::Md4, 0);
        assert_eq!(hex::encode(digest_md4), "7ced6b52c8203ba97580659d7dc33548",);

        let digest_md5 = strong_digest(b"hello world", StrongHash::Md5, 0);
        assert_eq!(hex::encode(digest_md5), "be4b47980f89d075f8f7e7a9fab84e29",);

        let digest_sha1 = strong_digest(b"hello world", StrongHash::Sha1, 0);
        assert_eq!(
            hex::encode(digest_sha1),
            "1fb6475c524899f98b088f7608bdab8f1591e078",
        );

        let digest_xxhash = strong_digest(b"hello world", StrongHash::XxHash, 0);
        assert_eq!(hex::encode(digest_xxhash), "68691eb23467ab45");
    }

    #[test]
    fn negotiate_prefers_upstream_order() {
        let local = available_strong_hashes();

        let cases = [
            (vec![StrongHash::Md4, StrongHash::Md5], StrongHash::Md5),
            (
                vec![StrongHash::Sha1, StrongHash::Md4, StrongHash::Md5],
                StrongHash::Md5,
            ),
            (
                vec![StrongHash::Md4, StrongHash::XxHash],
                StrongHash::XxHash,
            ),
        ];

        for (remote, expected) in cases {
            assert_eq!(
                negotiate_strong_hash(local, &remote),
                Some(expected),
                "remote list {:?} should negotiate to {:?}",
                remote,
                expected
            );
        }
    }
}
