// crates/checksums/src/lib.rs
//! Checksum algorithms for oc-rsync.
#![deny(unsafe_op_in_unsafe_fn, rust_2018_idioms)]
#![deny(warnings)]
#![warn(missing_docs)]

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

    pub fn strong_hasher(&self) -> StrongHasher {
        match self.strong {
            StrongHash::Md4 => StrongHasher {
                inner: StrongHasherInner::Md4(Md4::new()),
                seed: self.seed,
            },
            StrongHash::Md5 => {
                let mut h = Md5::new();
                h.update(self.seed.to_le_bytes());
                StrongHasher {
                    inner: StrongHasherInner::Md5(h),
                    seed: self.seed,
                }
            }
            StrongHash::Sha1 => {
                let mut h = Sha1::new();
                h.update(self.seed.to_le_bytes());
                StrongHasher {
                    inner: StrongHasherInner::Sha1(h),
                    seed: self.seed,
                }
            }
            StrongHash::XxHash => StrongHasher {
                inner: StrongHasherInner::XxHash(Xxh64::new(self.seed as u64)),
                seed: self.seed,
            },
        }
    }
}

pub struct StrongHasher {
    inner: StrongHasherInner,
    seed: u32,
}

enum StrongHasherInner {
    Md4(Md4),
    Md5(Md5),
    Sha1(Sha1),
    XxHash(Xxh64),
}

impl StrongHasher {
    pub fn update(&mut self, data: &[u8]) {
        match &mut self.inner {
            StrongHasherInner::Md4(h) => h.update(data),
            StrongHasherInner::Md5(h) => h.update(data),
            StrongHasherInner::Sha1(h) => h.update(data),
            StrongHasherInner::XxHash(h) => h.update(data),
        }
    }

    pub fn finalize(self) -> Vec<u8> {
        match self.inner {
            StrongHasherInner::Md4(mut h) => {
                h.update(self.seed.to_le_bytes());
                h.finalize().to_vec()
            }
            StrongHasherInner::Md5(h) => h.finalize().to_vec(),
            StrongHasherInner::Sha1(h) => h.finalize().to_vec(),
            StrongHasherInner::XxHash(h) => h.digest().to_le_bytes().to_vec(),
        }
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

pub fn rolling_checksum(data: &[u8]) -> u32 {
    rolling_checksum_seeded(data, 0)
}

pub fn rolling_checksum_seeded(data: &[u8], seed: u32) -> u32 {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(all(feature = "nightly", rustversion = "nightly"))]
        if std::arch::is_x86_feature_detected!("avx512f") {
            return unsafe { rolling_checksum_avx512(data, seed) };
        }
        if std::arch::is_x86_feature_detected!("avx2") {
            return unsafe { rolling_checksum_avx2(data, seed) };
        }
        if std::arch::is_x86_feature_detected!("sse4.2") {
            return unsafe { rolling_checksum_sse42(data, seed) };
        }
    }
    rolling_checksum_scalar(data, seed)
}

#[inline]
#[doc(hidden)]
pub fn rolling_checksum_scalar(data: &[u8], seed: u32) -> u32 {
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

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.2")]
#[doc(hidden)]
#[allow(unused_unsafe)]
pub unsafe fn rolling_checksum_sse42(data: &[u8], seed: u32) -> u32 {
    use std::arch::x86_64::*;

    const IDX_LO: [i16; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    const IDX_HI: [i16; 8] = [8, 9, 10, 11, 12, 13, 14, 15];

    let n = data.len();
    let mut sum_bytes: u64 = 0;
    let mut sum_indices: u64 = 0;
    let mut offset: u64 = 0;

    let zero = unsafe { _mm_setzero_si128() };
    let idx_lo = unsafe { _mm_loadu_si128(IDX_LO.as_ptr() as *const __m128i) };
    let idx_hi = unsafe { _mm_loadu_si128(IDX_HI.as_ptr() as *const __m128i) };

    let mut i = 0;
    while i + 16 <= n {
        let ptr = unsafe { data.as_ptr().add(i) };
        let chunk = unsafe { _mm_loadu_si128(ptr as *const __m128i) };

        let sad = unsafe { _mm_sad_epu8(chunk, zero) };
        let mut tmp_sum = [0u64; 2];
        unsafe { _mm_storeu_si128(tmp_sum.as_mut_ptr() as *mut __m128i, sad) };
        let chunk_sum = tmp_sum[0] + tmp_sum[1];
        sum_bytes += chunk_sum;

        let lo = unsafe { _mm_cvtepu8_epi16(chunk) };
        let shifted = unsafe { _mm_srli_si128(chunk, 8) };
        let hi = unsafe { _mm_cvtepu8_epi16(shifted) };
        let prod_lo = unsafe { _mm_madd_epi16(lo, idx_lo) };
        let prod_hi = unsafe { _mm_madd_epi16(hi, idx_hi) };
        let mut tmp = [0i32; 4];
        unsafe { _mm_storeu_si128(tmp.as_mut_ptr() as *mut __m128i, prod_lo) };
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        unsafe { _mm_storeu_si128(tmp.as_mut_ptr() as *mut __m128i, prod_hi) };
        let sum_hi: i64 = tmp.iter().map(|&v| v as i64).sum();
        let chunk_idx_sum = sum_lo + sum_hi;

        sum_indices += offset * chunk_sum + chunk_idx_sum as u64;

        offset += 16;
        i += 16;
    }

    for &b in &data[i..] {
        sum_bytes += b as u64;
        sum_indices += offset * b as u64;
        offset += 1;
    }

    let s1 = (sum_bytes as u32).wrapping_add(seed);
    let s2 = ((n as u64)
        .wrapping_mul(seed as u64)
        .wrapping_add((n as u64).wrapping_mul(sum_bytes).wrapping_sub(sum_indices)))
        as u32;
    (s1 & 0xffff) | (s2 << 16)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
#[doc(hidden)]
#[allow(unused_unsafe)]
pub unsafe fn rolling_checksum_avx2(data: &[u8], seed: u32) -> u32 {
    use std::arch::x86_64::*;

    const IDX_LO: [i16; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    const IDX_HI: [i16; 16] = [
        16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31,
    ];

    let n = data.len();
    let mut sum_bytes: u64 = 0;
    let mut sum_indices: u64 = 0;
    let mut offset: u64 = 0;

    let zero = unsafe { _mm256_setzero_si256() };
    let idx_lo = unsafe { _mm256_loadu_si256(IDX_LO.as_ptr() as *const __m256i) };
    let idx_hi = unsafe { _mm256_loadu_si256(IDX_HI.as_ptr() as *const __m256i) };

    let mut i = 0;
    while i + 32 <= n {
        let ptr = unsafe { data.as_ptr().add(i) };
        let chunk = unsafe { _mm256_loadu_si256(ptr as *const __m256i) };

        let sad = unsafe { _mm256_sad_epu8(chunk, zero) };
        let mut tmp_sum = [0u64; 4];
        unsafe { _mm256_storeu_si256(tmp_sum.as_mut_ptr() as *mut __m256i, sad) };
        let chunk_sum = tmp_sum.iter().sum::<u64>();
        sum_bytes += chunk_sum;

        let lower = unsafe { _mm256_castsi256_si128(chunk) };
        let upper = unsafe { _mm256_extracti128_si256(chunk, 1) };
        let lo = unsafe { _mm256_cvtepu8_epi16(lower) };
        let hi = unsafe { _mm256_cvtepu8_epi16(upper) };
        let prod_lo = unsafe { _mm256_madd_epi16(lo, idx_lo) };
        let prod_hi = unsafe { _mm256_madd_epi16(hi, idx_hi) };
        let mut tmp = [0i32; 8];
        unsafe { _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, prod_lo) };
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        unsafe { _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, prod_hi) };
        let sum_hi: i64 = tmp.iter().map(|&v| v as i64).sum();
        let chunk_idx_sum = sum_lo + sum_hi;

        sum_indices += offset * chunk_sum + chunk_idx_sum as u64;

        offset += 32;
        i += 32;
    }

    for &b in &data[i..] {
        sum_bytes += b as u64;
        sum_indices += offset * b as u64;
        offset += 1;
    }

    let s1 = (sum_bytes as u32).wrapping_add(seed);
    let s2 = ((n as u64)
        .wrapping_mul(seed as u64)
        .wrapping_add((n as u64).wrapping_mul(sum_bytes).wrapping_sub(sum_indices)))
        as u32;
    (s1 & 0xffff) | (s2 << 16)
}

#[cfg(all(
    feature = "nightly",
    rustversion = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[target_feature(enable = "avx512f,avx512bw")]
#[doc(hidden)]
#[allow(unused_unsafe)]
pub unsafe fn rolling_checksum_avx512(data: &[u8], seed: u32) -> u32 {
    use std::arch::x86_64::*;

    const IDX_LO: [i16; 32] = [
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31,
    ];
    const IDX_HI: [i16; 32] = [
        32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 52, 53, 54,
        55, 56, 57, 58, 59, 60, 61, 62, 63,
    ];

    let n = data.len();
    let mut sum_bytes: u64 = 0;
    let mut sum_indices: u64 = 0;
    let mut offset: u64 = 0;

    let zero = unsafe { _mm512_setzero_si512() };
    let idx_lo = unsafe { _mm512_loadu_si512(IDX_LO.as_ptr() as *const i32) };
    let idx_hi = unsafe { _mm512_loadu_si512(IDX_HI.as_ptr() as *const i32) };

    let mut i = 0;
    while i + 64 <= n {
        let ptr = unsafe { data.as_ptr().add(i) };
        let chunk = unsafe { _mm512_loadu_si512(ptr as *const i32) };

        let sad = unsafe { _mm512_sad_epu8(chunk, zero) };
        let mut tmp_sum = [0u64; 8];
        unsafe { _mm512_storeu_si512(tmp_sum.as_mut_ptr() as *mut i32, sad) };
        let chunk_sum = tmp_sum.iter().sum::<u64>();
        sum_bytes += chunk_sum;

        let lower = unsafe { _mm512_castsi512_si256(chunk) };
        let upper = unsafe { _mm512_extracti64x4_epi64(chunk, 1) };
        let lo = unsafe { _mm512_cvtepu8_epi16(lower) };
        let hi = unsafe { _mm512_cvtepu8_epi16(upper) };
        let prod_lo = unsafe { _mm512_madd_epi16(lo, idx_lo) };
        let prod_hi = unsafe { _mm512_madd_epi16(hi, idx_hi) };
        let mut tmp = [0i32; 16];
        unsafe { _mm512_storeu_si512(tmp.as_mut_ptr() as *mut i32, prod_lo) };
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        unsafe { _mm512_storeu_si512(tmp.as_mut_ptr() as *mut i32, prod_hi) };
        let sum_hi: i64 = tmp.iter().map(|&v| v as i64).sum();
        let chunk_idx_sum = sum_lo + sum_hi;

        sum_indices += offset * chunk_sum + chunk_idx_sum as u64;

        offset += 64;
        i += 64;
    }

    for &b in &data[i..] {
        sum_bytes += b as u64;
        sum_indices += offset * b as u64;
        offset += 1;
    }

    let s1 = (sum_bytes as u32).wrapping_add(seed);
    let s2 = ((n as u64)
        .wrapping_mul(seed as u64)
        .wrapping_add((n as u64).wrapping_mul(sum_bytes).wrapping_sub(sum_indices)))
        as u32;
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
    fn simd_equals_scalar() {
        let data = b"hello world";
        let scalar = rolling_checksum_scalar(data, 0);
        unsafe {
            assert_eq!(rolling_checksum_sse42(data, 0), scalar);
            assert_eq!(rolling_checksum_avx2(data, 0), scalar);
            #[cfg(all(feature = "nightly", rustversion = "nightly"))]
            assert_eq!(rolling_checksum_avx512(data, 0), scalar);
        }
    }

    #[test]
    fn simd_matches_scalar_varied() {
        let mut data = [0u8; 512];
        for (i, b) in data.iter_mut().enumerate() {
            *b = ((i as u32).wrapping_mul(31).wrapping_add(7)) as u8;
        }
        for len in 0..=data.len() {
            let slice = &data[..len];
            let scalar = rolling_checksum_scalar(slice, 1);
            unsafe {
                assert_eq!(rolling_checksum_sse42(slice, 1), scalar);
                assert_eq!(rolling_checksum_avx2(slice, 1), scalar);
                #[cfg(all(feature = "nightly", rustversion = "nightly"))]
                assert_eq!(rolling_checksum_avx512(slice, 1), scalar);
            }
        }
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
