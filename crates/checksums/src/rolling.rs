// crates/checksums/src/rolling.rs

#![doc = include_str!("../../../docs/rolling.md")]

pub trait RollingChecksum: Send + Sync {
    fn checksum(&self, data: &[u8], seed: u32) -> u32;
}

struct Scalar;

impl RollingChecksum for Scalar {
    fn checksum(&self, data: &[u8], seed: u32) -> u32 {
        rolling_checksum_scalar(data, seed)
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
struct Sse42;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl RollingChecksum for Sse42 {
    fn checksum(&self, data: &[u8], seed: u32) -> u32 {
        // SAFETY: CPU feature detection ensures SSE4.2 support and the data slice is valid.
        unsafe { rolling_checksum_sse42(data, seed) }
    }
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
struct Avx2;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
impl RollingChecksum for Avx2 {
    fn checksum(&self, data: &[u8], seed: u32) -> u32 {
        // SAFETY: CPU feature detection ensures AVX2 support and the data slice is valid.
        unsafe { rolling_checksum_avx2(data, seed) }
    }
}

#[cfg(all(
    feature = "nightly",
    rustversion = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
struct Avx512;

#[cfg(all(
    feature = "nightly",
    rustversion = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
impl RollingChecksum for Avx512 {
    fn checksum(&self, data: &[u8], seed: u32) -> u32 {
        // SAFETY: CPU feature detection ensures AVX512 support and the data slice is valid.
        unsafe { rolling_checksum_avx512(data, seed) }
    }
}

static SCALAR: Scalar = Scalar;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static SSE42: Sse42 = Sse42;

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
static AVX2: Avx2 = Avx2;

#[cfg(all(
    feature = "nightly",
    rustversion = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
static AVX512: Avx512 = Avx512;

pub fn rolling_checksum(data: &[u8]) -> u32 {
    rolling_checksum_seeded(data, 0)
}

pub fn rolling_checksum_seeded(data: &[u8], seed: u32) -> u32 {
    select_rolling_checksum().checksum(data, seed)
}

fn select_rolling_checksum() -> &'static dyn RollingChecksum {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        #[cfg(all(feature = "nightly", rustversion = "nightly"))]
        if std::arch::is_x86_feature_detected!("avx512f") {
            return &AVX512;
        }
        if std::arch::is_x86_feature_detected!("avx2") {
            return &AVX2;
        }
        if std::arch::is_x86_feature_detected!("sse4.2") {
            return &SSE42;
        }
    }
    &SCALAR
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
#[allow(unsafe_op_in_unsafe_fn)]
#[doc = "# Safety\n\
The calling CPU must support SSE4.2. Callers must verify support with\n\
`is_x86_feature_detected!(\"sse4.2\")` before invoking this function.\n\
The `data` slice must be valid for reads."]
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
        let ptr = data.as_ptr().add(i);
        let chunk = _mm_loadu_si128(ptr as *const __m128i);

        let sad = _mm_sad_epu8(chunk, zero);
        let mut tmp_sum = [0u64; 2];
        _mm_storeu_si128(tmp_sum.as_mut_ptr() as *mut __m128i, sad);
        let chunk_sum = tmp_sum[0] + tmp_sum[1];
        sum_bytes += chunk_sum;

        let lo = _mm_cvtepu8_epi16(chunk);
        let shifted = _mm_srli_si128(chunk, 8);
        let hi = _mm_cvtepu8_epi16(shifted);
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
#[allow(unused_unsafe)]
#[allow(unsafe_op_in_unsafe_fn)]
#[doc = "# Safety\n\
The calling CPU must support AVX2. Callers must verify support with\n\
`is_x86_feature_detected!(\"avx2\")` before invoking this function.\n\
The `data` slice must be valid for reads."]
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
        let ptr = data.as_ptr().add(i);
        let chunk = _mm256_loadu_si256(ptr as *const __m256i);

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

#[cfg(all(
    feature = "nightly",
    rustversion = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[target_feature(enable = "avx512f,avx512bw")]
#[doc(hidden)]
#[allow(unused_unsafe)]
#[allow(unsafe_op_in_unsafe_fn)]
#[doc = "# Safety\n\
Requires a CPU with the `avx512f` and `avx512bw` features. Callers must\n\
verify support with `is_x86_feature_detected!(\"avx512f\")` and\n\
`is_x86_feature_detected!(\"avx512bw\")` before invoking. The `data` slice\n\
must be valid for reads."]
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
    let idx_lo = _mm512_loadu_si512(IDX_LO.as_ptr() as *const i32);
    let idx_hi = _mm512_loadu_si512(IDX_HI.as_ptr() as *const i32);

    let mut i = 0;
    while i + 64 <= n {
        let ptr = data.as_ptr().add(i);
        let chunk = _mm512_loadu_si512(ptr as *const i32);

        let sad = _mm512_sad_epu8(chunk, zero);
        let mut tmp_sum = [0u64; 8];
        _mm512_storeu_si512(tmp_sum.as_mut_ptr() as *mut i32, sad);
        let chunk_sum = tmp_sum.iter().sum::<u64>();
        sum_bytes += chunk_sum;

        let lower = _mm512_castsi512_si256(chunk);
        let upper = _mm512_extracti64x4_epi64(chunk, 1);
        let lo = _mm512_cvtepu8_epi16(lower);
        let hi = _mm512_cvtepu8_epi16(upper);
        let prod_lo = _mm512_madd_epi16(lo, idx_lo);
        let prod_hi = _mm512_madd_epi16(hi, idx_hi);
        let mut tmp = [0i32; 16];
        _mm512_storeu_si512(tmp.as_mut_ptr() as *mut i32, prod_lo);
        let sum_lo: i64 = tmp.iter().map(|&v| v as i64).sum();
        _mm512_storeu_si512(tmp.as_mut_ptr() as *mut i32, prod_hi);
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
    fn simd_equals_scalar() {
        let data = b"hello world";
        let scalar = rolling_checksum_scalar(data, 0);
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            assert_eq!(SSE42.checksum(data, 0), scalar);
            assert_eq!(AVX2.checksum(data, 0), scalar);
            #[cfg(all(feature = "nightly", rustversion = "nightly"))]
            assert_eq!(AVX512.checksum(data, 0), scalar);
        }
    }

    #[test]
    fn simd_matches_scalar_varied() {
        let mut data = [0u8; 512];
        for (i, b) in data.iter_mut().enumerate() {
            *b = ((i as u32).wrapping_mul(31).wrapping_add(7)) as u8;
        }
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        for len in 0..=data.len() {
            let slice = &data[..len];
            let scalar = rolling_checksum_scalar(slice, 1);
            assert_eq!(SSE42.checksum(slice, 1), scalar);
            assert_eq!(AVX2.checksum(slice, 1), scalar);
            #[cfg(all(feature = "nightly", rustversion = "nightly"))]
            assert_eq!(AVX512.checksum(slice, 1), scalar);
        }
    }
}
