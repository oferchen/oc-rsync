// crates/compress/src/lib.rs
use std::io::{self, Read, Write};
use std::path::Path;

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
        #[cfg(feature = "nightly")]
        if std::arch::is_x86_feature_detected!("avx512f") {
            return true;
        }
        std::arch::is_x86_feature_detected!("avx2") || std::arch::is_x86_feature_detected!("sse4.2")
    }
    #[cfg(target_arch = "aarch64")]
    {
        std::arch::is_aarch64_feature_detected!("sve")
            || std::arch::is_aarch64_feature_detected!("neon")
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
    zstd::bulk::compress(data, level).map_err(io::Error::other)
}

#[inline]
fn zstd_decompress_scalar(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = zstd::stream::Decoder::new(data)?;
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.2")]
unsafe fn zstd_compress_sse42(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    use zstd::zstd_safe;
    let bound = zstd_safe::compress_bound(data.len());
    let mut out = vec![0u8; bound];
    let written =
        zstd_safe::compress(&mut out, data, level).map_err(|e| io::Error::other(format!("{e}")))?;
    out.truncate(written);
    Ok(out)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn zstd_compress_avx2(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    use zstd::zstd_safe;
    let bound = zstd_safe::compress_bound(data.len());
    let mut out = vec![0u8; bound];
    let written =
        zstd_safe::compress(&mut out, data, level).map_err(|e| io::Error::other(format!("{e}")))?;
    out.truncate(written);
    Ok(out)
}

#[cfg(all(feature = "nightly", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "avx512f")]
unsafe fn zstd_compress_avx512(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    use zstd::zstd_safe;
    let bound = zstd_safe::compress_bound(data.len());
    let mut out = vec![0u8; bound];
    let written =
        zstd_safe::compress(&mut out, data, level).map_err(|e| io::Error::other(format!("{e}")))?;
    out.truncate(written);
    Ok(out)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "sse4.2")]
unsafe fn zstd_decompress_sse42(data: &[u8]) -> io::Result<Vec<u8>> {
    use zstd::zstd_safe;
    let size = zstd_safe::get_frame_content_size(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unknown size"))?;
    let mut out = vec![0u8; size as usize];
    let written = zstd_safe::decompress(&mut out, data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?;
    out.truncate(written);
    Ok(out)
}

#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[target_feature(enable = "avx2")]
unsafe fn zstd_decompress_avx2(data: &[u8]) -> io::Result<Vec<u8>> {
    use zstd::zstd_safe;
    let size = zstd_safe::get_frame_content_size(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unknown size"))?;
    let mut out = vec![0u8; size as usize];
    let written = zstd_safe::decompress(&mut out, data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?;
    out.truncate(written);
    Ok(out)
}

#[cfg(all(feature = "nightly", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "avx512f")]
unsafe fn zstd_decompress_avx512(data: &[u8]) -> io::Result<Vec<u8>> {
    use zstd::zstd_safe;
    let size = zstd_safe::get_frame_content_size(data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "unknown size"))?;
    let mut out = vec![0u8; size as usize];
    let written = zstd_safe::decompress(&mut out, data)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, format!("{e}")))?;
    out.truncate(written);
    Ok(out)
}

impl Compressor for Zstd {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(feature = "nightly")]
            if std::arch::is_x86_feature_detected!("avx512f") {
                return unsafe { zstd_compress_avx512(data, self.level) };
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                return unsafe { zstd_compress_avx2(data, self.level) };
            }
            if std::arch::is_x86_feature_detected!("sse4.2") {
                return unsafe { zstd_compress_sse42(data, self.level) };
            }
        }
        zstd_compress_scalar(data, self.level)
    }
}

impl Decompressor for Zstd {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(feature = "nightly")]
            if std::arch::is_x86_feature_detected!("avx512f") {
                return unsafe { zstd_decompress_avx512(data) };
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                return unsafe { zstd_decompress_avx2(data) };
            }
            if std::arch::is_x86_feature_detected!("sse4.2") {
                return unsafe { zstd_decompress_sse42(data) };
            }
        }
        zstd_decompress_scalar(data)
    }
}

#[cfg(feature = "lz4")]
pub struct Lz4;

#[cfg(feature = "lz4")]
impl Compressor for Lz4 {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(feature = "nightly")]
            if std::arch::is_x86_feature_detected!("avx512f") {
                return unsafe { lz4_compress_avx512(data) };
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                return unsafe { lz4_compress_avx2(data) };
            }
            if std::arch::is_x86_feature_detected!("sse4.2") {
                return unsafe { lz4_compress_sse42(data) };
            }
        }
        lz4_compress_scalar(data)
    }
}

#[cfg(feature = "lz4")]
impl Decompressor for Lz4 {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        {
            #[cfg(feature = "nightly")]
            if std::arch::is_x86_feature_detected!("avx512f") {
                return unsafe { lz4_decompress_avx512(data) };
            }
            if std::arch::is_x86_feature_detected!("avx2") {
                return unsafe { lz4_decompress_avx2(data) };
            }
            if std::arch::is_x86_feature_detected!("sse4.2") {
                return unsafe { lz4_decompress_sse42(data) };
            }
        }
        lz4_decompress_scalar(data)
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
#[inline]
fn lz4_lib_compress(data: &[u8]) -> io::Result<Vec<u8>> {
    use lz4::liblz4::{LZ4_compressBound, LZ4_compress_default};
    use std::ffi::{c_char, c_int};

    let src_size = data.len() as c_int;
    let bound = unsafe { LZ4_compressBound(src_size) };
    let mut out = vec![0u8; 4 + bound as usize];
    let dst_ptr = out[4..].as_mut_ptr() as *mut c_char;
    let src_ptr = data.as_ptr() as *const c_char;
    let written = unsafe { LZ4_compress_default(src_ptr, dst_ptr, src_size, bound) };
    if written <= 0 {
        return Err(io::Error::other("LZ4_compress_default failed"));
    }
    out[..4].copy_from_slice(&(data.len() as u32).to_le_bytes());
    out.truncate(4 + written as usize);
    Ok(out)
}

#[cfg(feature = "lz4")]
#[inline]
fn lz4_lib_decompress(data: &[u8]) -> io::Result<Vec<u8>> {
    use lz4::liblz4::LZ4_decompress_safe;
    use std::ffi::{c_char, c_int};

    if data.len() < 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "input too short",
        ));
    }
    let (size_bytes, rest) = data.split_at(4);
    let size = u32::from_le_bytes(size_bytes.try_into().unwrap()) as c_int;
    let mut out = vec![0u8; size as usize];
    let src_ptr = rest.as_ptr() as *const c_char;
    let dst_ptr = out.as_mut_ptr() as *mut c_char;
    let decoded = unsafe { LZ4_decompress_safe(src_ptr, dst_ptr, rest.len() as c_int, size) };
    if decoded < 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "LZ4_decompress_safe failed",
        ));
    }
    Ok(out)
}

#[cfg(all(feature = "lz4", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "sse4.2")]
unsafe fn lz4_compress_sse42(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_lib_compress(data)
}

#[cfg(all(feature = "lz4", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "avx2")]
unsafe fn lz4_compress_avx2(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_lib_compress(data)
}

#[cfg(all(
    feature = "lz4",
    feature = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[target_feature(enable = "avx512f")]
unsafe fn lz4_compress_avx512(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_lib_compress(data)
}

#[cfg(all(feature = "lz4", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "sse4.2")]
unsafe fn lz4_decompress_sse42(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_lib_decompress(data)
}

#[cfg(all(feature = "lz4", any(target_arch = "x86", target_arch = "x86_64")))]
#[target_feature(enable = "avx2")]
unsafe fn lz4_decompress_avx2(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_lib_decompress(data)
}

#[cfg(all(
    feature = "lz4",
    feature = "nightly",
    any(target_arch = "x86", target_arch = "x86_64")
))]
#[target_feature(enable = "avx512f")]
unsafe fn lz4_decompress_avx512(data: &[u8]) -> io::Result<Vec<u8>> {
    lz4_lib_decompress(data)
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

    #[test]
    fn zstd_simd_matches_scalar() {
        let data = b"hello world";
        let level = 0;
        let scalar_c = zstd_compress_scalar(data, level).unwrap();
        unsafe {
            assert_eq!(zstd_compress_sse42(data, level).unwrap(), scalar_c);
            assert_eq!(zstd_compress_avx2(data, level).unwrap(), scalar_c);
            #[cfg(feature = "nightly")]
            assert_eq!(zstd_compress_avx512(data, level).unwrap(), scalar_c);
            let scalar_d = zstd_decompress_scalar(&scalar_c).unwrap();
            assert_eq!(zstd_decompress_sse42(&scalar_c).unwrap(), scalar_d);
            assert_eq!(zstd_decompress_avx2(&scalar_c).unwrap(), scalar_d);
            #[cfg(feature = "nightly")]
            assert_eq!(zstd_decompress_avx512(&scalar_c).unwrap(), scalar_d);
        }
    }

    #[cfg(feature = "lz4")]
    #[test]
    fn lz4_simd_matches_scalar() {
        let data = b"hello world";
        let scalar_c = lz4_compress_scalar(data).unwrap();
        unsafe {
            assert_eq!(lz4_compress_sse42(data).unwrap(), scalar_c);
            assert_eq!(lz4_compress_avx2(data).unwrap(), scalar_c);
            #[cfg(feature = "nightly")]
            assert_eq!(lz4_compress_avx512(data).unwrap(), scalar_c);
            let scalar_d = lz4_decompress_scalar(&scalar_c).unwrap();
            assert_eq!(lz4_decompress_sse42(&scalar_c).unwrap(), scalar_d);
            assert_eq!(lz4_decompress_avx2(&scalar_c).unwrap(), scalar_d);
            #[cfg(feature = "nightly")]
            assert_eq!(lz4_decompress_avx512(&scalar_c).unwrap(), scalar_d);
        }
    }
}
