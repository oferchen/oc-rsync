// crates/compress/src/lib.rs
use std::io::{self, Read, Write};
use std::path::Path;

cpufeatures::new!(sse42, "sse4.2");
cpufeatures::new!(avx2, "avx2");
cpufeatures::new!(avx512, "avx512f");
#[cfg(target_arch = "aarch64")]
cpufeatures::new!(neon, "neon");
#[cfg(target_arch = "aarch64")]
cpufeatures::new!(sve, "sve");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModernCompress {
    Auto,
    Zstd,
    Lz4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Zlib,
    Zstd,
    Lz4,
}

impl Codec {
    pub fn to_byte(self) -> u8 {
        match self {
            Codec::Zlib => 0,
            Codec::Zstd => 1,
            Codec::Lz4 => 2,
        }
    }

    pub fn from_byte(b: u8) -> io::Result<Self> {
        match b {
            0 => Ok(Codec::Zlib),
            1 => Ok(Codec::Zstd),
            2 => Ok(Codec::Lz4),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown codec {other}"),
            )),
        }
    }
}

fn has_zstd_simd() -> bool {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        avx512::get() || avx2::get()
    }
    #[cfg(target_arch = "aarch64")]
    {
        neon::get() || sve::get()
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64", target_arch = "aarch64")))]
    {
        false
    }
}

fn auto_codec(has_simd: bool) -> Option<Codec> {
    if has_simd {
        Some(Codec::Zstd)
    } else {
        #[cfg(feature = "lz4")]
        {
            Some(Codec::Lz4)
        }
        #[cfg(not(feature = "lz4"))]
        {
            None
        }
    }
}

pub fn available_codecs(modern: Option<ModernCompress>) -> Vec<Codec> {
    let mut codecs = vec![Codec::Zlib];
    if let Some(mode) = modern {
        match mode {
            ModernCompress::Auto => {
                if let Some(codec) = auto_codec(has_zstd_simd()) {
                    codecs.push(codec);
                }
            }
            ModernCompress::Zstd => {
                codecs.push(Codec::Zstd);
            }
            ModernCompress::Lz4 => {
                #[cfg(feature = "lz4")]
                {
                    codecs.push(Codec::Lz4);
                }
            }
        }
    }
    codecs
}

pub trait Compressor {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>>;
}

pub trait Decompressor {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>>;
}

pub fn negotiate_codec(local: &[Codec], remote: &[Codec]) -> Option<Codec> {
    local.iter().copied().find(|c| remote.contains(c))
}

pub fn encode_codecs(codecs: &[Codec]) -> Vec<u8> {
    codecs.iter().map(|c| c.to_byte()).collect()
}

pub fn decode_codecs(data: &[u8]) -> io::Result<Vec<Codec>> {
    data.iter().map(|b| Codec::from_byte(*b)).collect()
}

pub fn should_compress(path: &Path, skip: &[String]) -> bool {
    if skip.is_empty() {
        return true;
    }
    match path.file_name().and_then(|n| n.to_str()) {
        Some(name) => !skip.iter().any(|s| name.ends_with(s)),
        None => true,
    }
}

pub struct Zlib {
    level: i32,
}

impl Zlib {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

impl Default for Zlib {
    fn default() -> Self {
        Self { level: 6 }
    }
}

impl Compressor for Zlib {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        let mut encoder = flate2::write::ZlibEncoder::new(
            Vec::new(),
            flate2::Compression::new(self.level as u32),
        );
        encoder.write_all(data)?;
        encoder.finish()
    }
}

impl Decompressor for Zlib {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        let mut decoder = flate2::read::ZlibDecoder::new(data);
        let mut out = Vec::new();
        decoder.read_to_end(&mut out)?;
        Ok(out)
    }
}

#[derive(Default)]
pub struct Zstd {
    level: i32,
}

impl Zstd {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

#[inline]
fn zstd_compress_scalar(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    let mut encoder = zstd::stream::Encoder::new(Vec::new(), level)?;
    encoder.write_all(data)?;
    encoder.finish()
}

#[inline]
fn zstd_decompress_scalar(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = zstd::stream::Decoder::new(data)?;
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

#[target_feature(enable = "sse4.2")]
unsafe fn zstd_compress_sse42(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    zstd_compress_scalar(data, level)
}

#[target_feature(enable = "avx2")]
unsafe fn zstd_compress_avx2(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    zstd_compress_scalar(data, level)
}

#[target_feature(enable = "avx512f")]
unsafe fn zstd_compress_avx512(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    zstd_compress_scalar(data, level)
}

#[target_feature(enable = "sse4.2")]
unsafe fn zstd_decompress_sse42(data: &[u8]) -> io::Result<Vec<u8>> {
    zstd_decompress_scalar(data)
}

#[target_feature(enable = "avx2")]
unsafe fn zstd_decompress_avx2(data: &[u8]) -> io::Result<Vec<u8>> {
    zstd_decompress_scalar(data)
}

#[target_feature(enable = "avx512f")]
unsafe fn zstd_decompress_avx512(data: &[u8]) -> io::Result<Vec<u8>> {
    zstd_decompress_scalar(data)
}

impl Compressor for Zstd {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if avx512::get() {
            unsafe { zstd_compress_avx512(data, self.level) }
        } else if avx2::get() {
            unsafe { zstd_compress_avx2(data, self.level) }
        } else if sse42::get() {
            unsafe { zstd_compress_sse42(data, self.level) }
        } else {
            zstd_compress_scalar(data, self.level)
        }
    }
}

impl Decompressor for Zstd {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if avx512::get() {
            unsafe { zstd_decompress_avx512(data) }
        } else if avx2::get() {
            unsafe { zstd_decompress_avx2(data) }
        } else if sse42::get() {
            unsafe { zstd_decompress_sse42(data) }
        } else {
            zstd_decompress_scalar(data)
        }
    }
}

#[cfg(feature = "lz4")]
pub struct Lz4;

#[cfg(feature = "lz4")]
impl Compressor for Lz4 {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if avx512::get() {
            unsafe { lz4_compress_avx512(data) }
        } else if avx2::get() {
            unsafe { lz4_compress_avx2(data) }
        } else if sse42::get() {
            unsafe { lz4_compress_sse42(data) }
        } else {
            lz4_compress_scalar(data)
        }
    }
}

#[cfg(feature = "lz4")]
impl Decompressor for Lz4 {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        if avx512::get() {
            unsafe { lz4_decompress_avx512(data) }
        } else if avx2::get() {
            unsafe { lz4_decompress_avx2(data) }
        } else if sse42::get() {
            unsafe { lz4_decompress_sse42(data) }
        } else {
            lz4_decompress_scalar(data)
        }
    }
}

#[cfg(feature = "lz4")]
#[inline]
fn lz4_compress_scalar(data: &[u8]) -> io::Result<Vec<u8>> {
    Ok(lz4_flex::block::compress_prepend_size(data))
}

#[cfg(feature = "lz4")]
#[inline]
fn lz4_decompress_scalar(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_flex::block::decompress_size_prepended(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(feature = "lz4")]
#[target_feature(enable = "sse4.2")]
unsafe fn lz4_compress_sse42(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_compress_scalar(data)
}

#[cfg(feature = "lz4")]
#[target_feature(enable = "avx2")]
unsafe fn lz4_compress_avx2(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_compress_scalar(data)
}

#[cfg(feature = "lz4")]
#[target_feature(enable = "avx512f")]
unsafe fn lz4_compress_avx512(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_compress_scalar(data)
}

#[cfg(feature = "lz4")]
#[target_feature(enable = "sse4.2")]
unsafe fn lz4_decompress_sse42(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_decompress_scalar(data)
}

#[cfg(feature = "lz4")]
#[target_feature(enable = "avx2")]
unsafe fn lz4_decompress_avx2(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_decompress_scalar(data)
}

#[cfg(feature = "lz4")]
#[target_feature(enable = "avx512f")]
unsafe fn lz4_decompress_avx512(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_decompress_scalar(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_chooses_zstd_when_simd() {
        assert_eq!(auto_codec(true), Some(Codec::Zstd));
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn auto_falls_back_to_lz4_without_simd() {
        assert_eq!(auto_codec(false), Some(Codec::Lz4));
    }

    #[cfg(not(feature = "lz4"))]
    #[test]
    fn auto_disables_without_simd() {
        assert_eq!(auto_codec(false), None);
    }

    #[test]
    fn available_codecs_respects_simd_detection() {
        let mut expected = vec![Codec::Zlib];
        if let Some(c) = auto_codec(has_zstd_simd()) {
            expected.push(c);
        }
        assert_eq!(available_codecs(Some(ModernCompress::Auto)), expected);
    }
}
