// crates/compress/src/zlib.rs
use std::io::{self, Read, Write};

use crate::{Compressor, Decompressor};

#[cfg(feature = "zlib")]
#[derive(Clone, Copy)]
pub struct Zlib {
    level: i32,
}

#[cfg(feature = "zlib")]
impl Zlib {
    pub fn new(level: i32) -> Self {
        let level = level.clamp(0, 9);
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
#[derive(Clone, Copy)]
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
