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
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(feature = "nightly")]
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
pub unsafe fn rolling_checksum_sse42(data: &[u8], seed: u32) -> u32 {
    use std::arch::x86_64::*;

    const IDX_LO: [i16; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
    const IDX_HI: [i16; 8] = [8, 9, 10, 11, 12, 13, 14, 15];

    let n = data.len();
    let mut sum_bytes: u64 = 0;
    let mut sum_indices: u64 = 0;
    let mut offset: u64 = 0;

    let zero = _mm_setzero_si128();
    let idx_lo = _mm_loadu_si128(IDX_LO.as_ptr() as *const __m128i);
    let idx_hi = _mm_loadu_si128(IDX_HI.as_ptr() as *const __m128i);

    let mut i = 0;
    while i + 16 <= n {
        let chunk = _mm_loadu_si128(data.as_ptr().add(i) as *const __m128i);

        let sad = _mm_sad_epu8(chunk, zero);
        let mut tmp_sum = [0u64; 2];
        _mm_storeu_si128(tmp_sum.as_mut_ptr() as *mut __m128i, sad);
        let chunk_sum = tmp_sum[0] + tmp_sum[1];
        sum_bytes += chunk_sum;

        let lo = _mm_cvtepu8_epi16(chunk);
        let hi = _mm_cvtepu8_epi16(_mm_srli_si128(chunk, 8));
        let prod_lo = _mm_madd_epi16(lo, idx_lo);
        let prod_hi = _mm_madd_epi16(hi, idx_hi);
        let mut tmp = [0i32; 4];
        _mm_storeu_si128(tmp.as_mut_ptr() as *mut __m128i, prod_lo);
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        _mm_storeu_si128(tmp.as_mut_ptr() as *mut __m128i, prod_hi);
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

    let zero = _mm256_setzero_si256();
    let idx_lo = _mm256_loadu_si256(IDX_LO.as_ptr() as *const __m256i);
    let idx_hi = _mm256_loadu_si256(IDX_HI.as_ptr() as *const __m256i);

    let mut i = 0;
    while i + 32 <= n {
        let chunk = _mm256_loadu_si256(data.as_ptr().add(i) as *const __m256i);

        let sad = _mm256_sad_epu8(chunk, zero);
        let mut tmp_sum = [0u64; 4];
        _mm256_storeu_si256(tmp_sum.as_mut_ptr() as *mut __m256i, sad);
        let chunk_sum = tmp_sum.iter().sum::<u64>();
        sum_bytes += chunk_sum;

        let lower = _mm256_castsi256_si128(chunk);
        let upper = _mm256_extracti128_si256(chunk, 1);
        let lo = _mm256_cvtepu8_epi16(lower);
        let hi = _mm256_cvtepu8_epi16(upper);
        let prod_lo = _mm256_madd_epi16(lo, idx_lo);
        let prod_hi = _mm256_madd_epi16(hi, idx_hi);
        let mut tmp = [0i32; 8];
        _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, prod_lo);
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        _mm256_storeu_si256(tmp.as_mut_ptr() as *mut __m256i, prod_hi);
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

#[cfg(all(feature = "nightly", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "avx512f,avx512bw")]
#[doc(hidden)]
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

    let zero = _mm512_setzero_si512();
    let idx_lo = _mm512_loadu_si512(IDX_LO.as_ptr() as *const __m512i);
    let idx_hi = _mm512_loadu_si512(IDX_HI.as_ptr() as *const __m512i);

    let mut i = 0;
    while i + 64 <= n {
        let chunk = _mm512_loadu_si512(data.as_ptr().add(i) as *const __m512i);

        let sad = _mm512_sad_epu8(chunk, zero);
        let mut tmp_sum = [0u64; 8];
        _mm512_storeu_si512(tmp_sum.as_mut_ptr() as *mut __m512i, sad);
        let chunk_sum = tmp_sum.iter().sum::<u64>();
        sum_bytes += chunk_sum;

        let lower = _mm512_castsi512_si256(chunk);
        let upper = _mm512_extracti64x4_epi64(chunk, 1);
        let lo = _mm512_cvtepu8_epi16(lower);
        let hi = _mm512_cvtepu8_epi16(upper);
        let prod_lo = _mm512_madd_epi16(lo, idx_lo);
        let prod_hi = _mm512_madd_epi16(hi, idx_hi);
        let mut tmp = [0i32; 16];
        _mm512_storeu_si512(tmp.as_mut_ptr() as *mut __m512i, prod_lo);
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        _mm512_storeu_si512(tmp.as_mut_ptr() as *mut __m512i, prod_hi);
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

    #[test]
    fn simd_equals_scalar() {
        let data = b"hello world";
        let scalar = rolling_checksum_scalar(data, 0);
        unsafe {
            assert_eq!(rolling_checksum_sse42(data, 0), scalar);
            assert_eq!(rolling_checksum_avx2(data, 0), scalar);
            #[cfg(feature = "nightly")]
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
                #[cfg(feature = "nightly")]
                assert_eq!(rolling_checksum_avx512(slice, 1), scalar);
            }
        }
    }
}
