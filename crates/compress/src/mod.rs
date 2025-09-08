// crates/compress/src/mod.rs
use std::collections::HashSet;
use std::io::{self, Read, Write};
use std::path::Path;
use std::sync::LazyLock;

#[cfg(feature = "zlib")]
pub mod zlib;
#[cfg(feature = "zstd")]
pub mod zstd;

#[cfg(feature = "zlib")]
pub use zlib::{Zlib, ZlibX};
#[cfg(feature = "zstd")]
pub use zstd::Zstd;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Zlib,
    ZlibX,
    Zstd,
}

impl Codec {
    pub fn to_byte(self) -> u8 {
        match self {
            Codec::Zlib => 1,
            Codec::ZlibX => 2,
            Codec::Zstd => 4,
        }
    }

    pub fn from_byte(b: u8) -> io::Result<Self> {
        match b {
            1 => Ok(Codec::Zlib),
            2 => Ok(Codec::ZlibX),
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
        Codec::ZlibX,
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

pub fn compressor(codec: Codec) -> io::Result<Box<dyn Compressor>> {
    match codec {
        #[cfg(feature = "zlib")]
        Codec::Zlib => Ok(Box::new(Zlib::default())),
        #[cfg(feature = "zlib")]
        Codec::ZlibX => Ok(Box::new(ZlibX::default())),
        #[cfg(feature = "zstd")]
        Codec::Zstd => Ok(Box::new(Zstd::default())),
        #[cfg(not(all(feature = "zlib", feature = "zstd")))]
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "codec not available",
        )),
    }
}

pub fn decompressor(codec: Codec) -> io::Result<Box<dyn Decompressor>> {
    match codec {
        #[cfg(feature = "zlib")]
        Codec::Zlib => Ok(Box::new(Zlib::default())),
        #[cfg(feature = "zlib")]
        Codec::ZlibX => Ok(Box::new(Zlib::default())),
        #[cfg(feature = "zstd")]
        Codec::Zstd => Ok(Box::new(Zstd::default())),
        #[cfg(not(all(feature = "zlib", feature = "zstd")))]
        _ => Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "codec not available",
        )),
    }
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

static DEFAULT_SKIP_COMPRESS_SET: LazyLock<HashSet<&'static str>> =
    LazyLock::new(|| DEFAULT_SKIP_COMPRESS.iter().copied().collect());

pub fn should_compress(path: &Path, skip: &HashSet<String>) -> bool {
    let ext = match path.extension().and_then(|e| e.to_str()) {
        Some(ext) => ext.to_ascii_lowercase(),
        None => return true,
    };

    if skip.is_empty() {
        return !DEFAULT_SKIP_COMPRESS_SET.contains(ext.as_str());
    }

    !skip.contains(ext.as_str())
}
