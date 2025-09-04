// crates/compress/src/lz4.rs

use std::io;

use crate::{Compressor, Decompressor};

#[derive(Default)]
pub struct Lz4;

impl Lz4 {
    pub fn new() -> Self {
        Self
    }
}

impl Compressor for Lz4 {
    fn compress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        Ok(lz4_flex::block::compress_prepend_size(data))
    }
}

impl Decompressor for Lz4 {
    fn decompress(&self, data: &[u8]) -> io::Result<Vec<u8>> {
        lz4_flex::block::decompress_size_prepended(data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }
}
