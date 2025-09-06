// crates/engine/src/flist.rs

use filelist::{Decoder, Encoder, Entry};
use protocol::CharsetConv;

pub fn encode(entries: &[Entry], iconv: Option<&CharsetConv>) -> Vec<Vec<u8>> {
    let mut enc = Encoder::new();
    entries
        .iter()
        .map(|e| {
            let mut e = e.clone();
            if let Some(cv) = iconv {
                e.path = cv.to_remote(&e.path).into_owned();
            }
            enc.encode_entry(&e)
        })
        .collect()
}

pub fn decode(
    chunks: &[Vec<u8>],
    iconv: Option<&CharsetConv>,
) -> Result<Vec<Entry>, filelist::DecodeError> {
    let mut dec = Decoder::new();
    chunks
        .iter()
        .map(|c| {
            let mut e = dec.decode_entry(c)?;
            if let Some(cv) = iconv {
                e.path = cv.to_local(&e.path).into_owned();
            }
            Ok(e)
        })
        .collect()
}
