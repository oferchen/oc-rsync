// crates/checksums/src/lib.rs

pub mod rolling;
pub mod strong;

pub use rolling::{Rolling, RollingChecksum, rolling_checksum, rolling_checksum_seeded};
pub use strong::{
    StrongChecksum, StrongHash, available_strong_hashes, negotiate_strong_hash, strong_digest,
};

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
            strong: StrongHash::Md4,
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

    pub fn negotiate(mut self, remote: &[StrongHash]) -> Self {
        if let Some(alg) = negotiate_strong_hash(available_strong_hashes(), remote) {
            self.strong = alg;
        }
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

    pub fn strong_hasher(&self) -> Box<dyn StrongChecksum> {
        strong::select_strong_checksum(self.strong, self.seed)
    }
}
