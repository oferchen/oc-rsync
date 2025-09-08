// crates/compress/src/zstd.rs
use std::io::{self, Read, Write};

use crate::{Compressor, Decompressor};

#[cfg(feature = "zstd")]
#[derive(Clone, Copy, Default)]
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
