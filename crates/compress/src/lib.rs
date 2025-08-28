use std::io::{self, Read, Write};

/// Supported compression codecs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// Deflate/zlib compression.
    Zlib,
    /// Zstandard compression.
    Zstd,
    /// LZ4 compression. Requires the `lz4` feature.
    Lz4,
}

/// Return codecs supported by this crate in preference order.
///
/// LZ4 is not advertised unless the `lz4` feature is enabled.
pub fn available_codecs() -> &'static [Codec] {
    #[cfg(feature = "lz4")]
    {
        &[Codec::Zlib, Codec::Zstd, Codec::Lz4]
    }
    #[cfg(not(feature = "lz4"))]
    {
        &[Codec::Zlib, Codec::Zstd]
    }
}

/// Compresses a buffer of bytes.
pub trait Compressor {
    /// Compress `data` and return the compressed bytes.
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>>;
}

/// Decompresses a buffer of bytes.
pub trait Decompressor {
    /// Decompress `data` and return the decompressed bytes.
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>>;
}

/// Select the first codec supported by both peers.
///
/// Typically the local list is provided by [`available_codecs`], ensuring that
/// only codecs enabled in this build are considered.
pub fn negotiate_codec(local: &[Codec], remote: &[Codec]) -> Option<Codec> {
    local.iter().copied().find(|c| remote.contains(c))
}

/// Zlib/Deflate codec adapter.
pub struct Zlib {
    level: i32,
}

impl Zlib {
    /// Create a new zlib codec with the given compression level.
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

/// Zstandard codec adapter.
pub struct Zstd {
    level: i32,
}

impl Zstd {
    /// Create a new zstd codec with the given compression level.
    pub fn new(level: i32) -> Self {
        Self { level }
    }
}

impl Default for Zstd {
    fn default() -> Self {
        Self { level: 0 }
    }
}

impl Compressor for Zstd {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        let mut encoder = zstd::stream::Encoder::new(Vec::new(), self.level)?;
        encoder.write_all(data)?;
        encoder.finish()
    }
}

impl Decompressor for Zstd {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        let mut decoder = zstd::stream::Decoder::new(data)?;
        let mut out = Vec::new();
        decoder.read_to_end(&mut out)?;
        Ok(out)
    }
}

#[cfg(feature = "lz4")]
/// LZ4 codec adapter.
pub struct Lz4;

#[cfg(feature = "lz4")]
impl Compressor for Lz4 {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        Ok(lz4_flex::block::compress_prepend_size(data))
    }
}

#[cfg(feature = "lz4")]
impl Decompressor for Lz4 {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        lz4_flex::block::decompress_size_prepended(data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
