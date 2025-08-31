// crates/engine/src/flist.rs
//! File list helpers built on top of the `filelist` crate.

use filelist::{Decoder, Encoder, Entry};

/// Encode a slice of entries into a vector of payloads suitable for
/// `protocol::Message::FileListEntry`.
pub fn encode(entries: &[Entry]) -> Vec<Vec<u8>> {
    let mut enc = Encoder::new();
    entries.iter().map(|e| enc.encode_entry(e)).collect()
}

/// Decode a list of payloads into file list entries.
pub fn decode(chunks: &[Vec<u8>]) -> Result<Vec<Entry>, filelist::DecodeError> {
    let mut dec = Decoder::new();
    chunks.iter().map(|c| dec.decode_entry(c)).collect()
}
