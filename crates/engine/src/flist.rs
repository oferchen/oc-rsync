// crates/engine/src/flist.rs

use filelist::{Decoder, Encoder, Entry};

pub fn encode(entries: &[Entry]) -> Vec<Vec<u8>> {
    let mut enc = Encoder::new();
    entries.iter().map(|e| enc.encode_entry(e)).collect()
}

pub fn decode(chunks: &[Vec<u8>]) -> Result<Vec<Entry>, filelist::DecodeError> {
    let mut dec = Decoder::new();
    chunks.iter().map(|c| dec.decode_entry(c)).collect()
}
