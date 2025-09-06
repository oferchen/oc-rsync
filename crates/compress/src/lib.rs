// crates/compress/src/lib.rs
use std::io::{self, Read, Write};

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Zlib,
    Zlibx,
    Zstd,
}

impl Codec {
    pub fn to_byte(self) -> u8 {
        match self {
            Codec::Zlib => 1,
            Codec::Zlibx => 2,
            Codec::Zstd => 4,
        }
    }

    pub fn from_byte(b: u8) -> io::Result<Self> {
        match b {
            1 => Ok(Codec::Zlib),
            2 => Ok(Codec::Zlibx),
            4 => Ok(Codec::Zstd),
            other => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("unknown codec {other}"),
            )),
        }
    }
}

pub fn available_codecs() -> Vec<Codec> {
    let codecs = [
        #[cfg(feature = "zstd")]
        Codec::Zstd,
        #[cfg(feature = "zlib")]
        Codec::Zlibx,
        #[cfg(feature = "zlib")]
        Codec::Zlib,
    ];
    codecs.into_iter().collect()
}

pub trait Compressor {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()>;
}

pub trait Decompressor {
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()>;
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

pub const DEFAULT_SKIP_COMPRESS: &[&str] = &[
    "3g2", "3gp", "7z", "aac", "ace", "apk", "avi", "bz2", "deb", "dmg", "ear", "f4v", "flac",
    "flv", "gpg", "gz", "iso", "jar", "jpeg", "jpg", "lrz", "lz", "lz4", "lzma", "lzo", "m1a",
    "m1v", "m2a", "m2ts", "m2v", "m4a", "m4b", "m4p", "m4r", "m4v", "mka", "mkv", "mov", "mp1",
    "mp2", "mp3", "mp4", "mpa", "mpeg", "mpg", "mpv", "mts", "odb", "odf", "odg", "odi", "odm",
    "odp", "ods", "odt", "oga", "ogg", "ogm", "ogv", "ogx", "opus", "otg", "oth", "otp", "ots",
    "ott", "oxt", "png", "qt", "rar", "rpm", "rz", "rzip", "spx", "squashfs", "sxc", "sxd", "sxg",
    "sxm", "sxw", "sz", "tbz", "tbz2", "tgz", "tlz", "ts", "txz", "tzo", "vob", "war", "webm",
    "webp", "xz", "z", "zip", "zst",
];

pub fn should_compress(path: &Path, skip: &[String]) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.to_ascii_lowercase(),
        None => return true,
    };

    if skip.is_empty() {
        return !DEFAULT_SKIP_COMPRESS.iter().any(|s| ext == *s);
    }

    !skip.iter().any(|s| ext == s.to_ascii_lowercase())
}

#[cfg(feature = "zlib")]
pub struct Zlib {
    level: i32,
}

#[cfg(feature = "zlib")]
impl Zlib {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

#[cfg(feature = "zlib")]
impl Default for Zlib {
    fn default() -> Self {
        Self { level: 6 }
    }
}

#[cfg(feature = "zlib")]
impl Compressor for Zlib {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()> {
        let mut encoder =
            flate2::write::ZlibEncoder::new(output, flate2::Compression::new(self.level as u32));
        io::copy(input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }
}

#[cfg(feature = "zlib")]
impl Decompressor for Zlib {
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()> {
        let mut decoder = flate2::read::ZlibDecoder::new(input);
        io::copy(&mut decoder, output)?;
        Ok(())
    }
}

#[cfg(feature = "zlib")]
pub struct ZlibX {
    level: i32,
}

#[cfg(feature = "zlib")]
impl ZlibX {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

#[cfg(feature = "zlib")]
impl Default for ZlibX {
    fn default() -> Self {
        Self { level: 6 }
    }
}

#[cfg(feature = "zlib")]
impl Compressor for ZlibX {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()> {
        Zlib::new(self.level).compress(input, output)
    }
}

#[cfg(feature = "zlib")]
impl Decompressor for ZlibX {
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()> {
        Zlib::default().decompress(input, output)
    }
}

#[cfg(feature = "zstd")]
#[derive(Default)]
pub struct Zstd {
    level: i32,
}

#[cfg(feature = "zstd")]
impl Zstd {
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

#[cfg(feature = "zstd")]
#[inline]
#[allow(dead_code)]
fn zstd_compress_scalar(data: &[u8], level: i32) -> io::Result<Vec<u8>> {
    zstd::bulk::compress(data, level).map_err(io::Error::other)
}

#[cfg(feature = "zstd")]
#[inline]
#[allow(dead_code)]
fn zstd_decompress_scalar(data: &[u8]) -> io::Result<Vec<u8>> {
    let mut decoder = zstd::stream::Decoder::new(data)?;
    let mut out = Vec::new();
    decoder.read_to_end(&mut out)?;
    Ok(out)
}

#[cfg(feature = "zstd")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
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

#[cfg(feature = "zstd")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
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

#[cfg(feature = "zstd")]
#[cfg(all(feature = "nightly", any(target_arch = "x86", target_arch = "x86_64")))]
#[allow(dead_code)]
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

#[cfg(feature = "zstd")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
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

#[cfg(feature = "zstd")]
#[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
#[allow(dead_code)]
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

#[cfg(feature = "zstd")]
#[cfg(all(feature = "nightly", any(target_arch = "x86", target_arch = "x86_64")))]
#[allow(dead_code)]
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

#[cfg(feature = "zstd")]
impl Compressor for Zstd {
    fn compress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()> {
        let mut encoder = zstd::stream::write::Encoder::new(output, self.level)?;
        io::copy(input, &mut encoder)?;
        encoder.finish()?;
        Ok(())
    }
}

#[cfg(feature = "zstd")]
impl Decompressor for Zstd {
    fn decompress(&self, input: &mut dyn Read, output: &mut dyn Write) -> io::Result<()> {
        let mut decoder = zstd::stream::write::Decoder::new(output)?;
        io::copy(input, &mut decoder)?;
        decoder.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "zstd")]
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

    #[cfg(all(feature = "zlib", feature = "zstd"))]
    #[test]
    fn available_codecs_returns_all_codecs() {
        assert_eq!(
            available_codecs(),
            vec![Codec::Zstd, Codec::Zlibx, Codec::Zlib]
        );
    }

    #[cfg(all(not(feature = "zlib"), feature = "zstd"))]
    #[test]
    fn available_codecs_returns_only_zstd() {
        assert_eq!(available_codecs(), vec![Codec::Zstd]);
    }

    #[cfg(all(feature = "zlib", not(feature = "zstd")))]
    #[test]
    fn available_codecs_returns_only_zlib() {
        assert_eq!(available_codecs(), vec![Codec::Zlibx, Codec::Zlib]);
    }

    #[cfg(all(not(feature = "zlib"), not(feature = "zstd")))]
    #[test]
    fn available_codecs_returns_empty() {
        assert!(available_codecs().is_empty());
    }
}
