// crates/filelist/src/lib.rs

use std::collections::HashMap;
use std::io::Read;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: String,
    pub uid: u32,
    pub gid: u32,
}

#[derive(Debug, Default)]
pub struct Encoder {
    prev_path: String,
    uid_table: HashMap<u32, u8>,
    gid_table: HashMap<u32, u8>,
}

#[derive(Debug, Default)]
pub struct Decoder {
    prev_path: String,
    uid_table: Vec<u32>,
    gid_table: Vec<u32>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("input too short")]
    ShortInput,
    #[error("invalid utf8")]
    Utf8,
    #[error("unknown uid index {0}")]
    BadUid(u8),
    #[error("unknown gid index {0}")]
    BadGid(u8),
}

impl Encoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn encode_entry(&mut self, entry: &Entry) -> Vec<u8> {
        let mut out = Vec::new();
        let common = common_prefix(&self.prev_path, &entry.path) as u8;
        let suffix = &entry.path[common as usize..];
        out.push(common);
        out.push(suffix.len() as u8);
        out.extend_from_slice(suffix.as_bytes());
        out.extend_from_slice(&encode_id(entry.uid, &mut self.uid_table));
        out.extend_from_slice(&encode_id(entry.gid, &mut self.gid_table));
        self.prev_path = entry.path.clone();
        out
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode_entry(&mut self, mut input: &[u8]) -> Result<Entry, DecodeError> {
        if input.len() < 2 {
            return Err(DecodeError::ShortInput);
        }
        let common = input[0] as usize;
        let suff_len = input[1] as usize;
        input = &input[2..];
        if input.len() < suff_len {
            return Err(DecodeError::ShortInput);
        }
        let suffix = &input[..suff_len];
        input = &input[suff_len..];
        let path_bytes: Vec<u8> = self.prev_path.as_bytes()[..common]
            .iter()
            .copied()
            .chain(suffix.iter().copied())
            .collect();
        let path = String::from_utf8(path_bytes).map_err(|_| DecodeError::Utf8)?;
        let (uid, rest) = decode_id(input, &mut self.uid_table, true)?;
        let (gid, rest) = decode_id(rest, &mut self.gid_table, false)?;
        let _ = rest;
        self.prev_path = path.clone();
        Ok(Entry { path, uid, gid })
    }
}

fn common_prefix(a: &str, b: &str) -> usize {
    a.bytes().zip(b.bytes()).take_while(|(x, y)| x == y).count()
}

fn encode_id(id: u32, table: &mut HashMap<u32, u8>) -> Vec<u8> {
    if let Some(&idx) = table.get(&id) {
        vec![idx]
    } else {
        let idx = table.len() as u8;
        table.insert(id, idx);
        let mut out = vec![0xFF];
        out.extend_from_slice(&id.to_le_bytes());
        out
    }
}

fn decode_id<'a>(
    mut input: &'a [u8],
    table: &mut Vec<u32>,
    is_uid: bool,
) -> Result<(u32, &'a [u8]), DecodeError> {
    if input.is_empty() {
        return Err(DecodeError::ShortInput);
    }
    let tag = input[0];
    input = &input[1..];
    if tag == 0xFF {
        if input.len() < 4 {
            return Err(DecodeError::ShortInput);
        }
        let mut rdr = &input[..4];
        let mut buf = [0u8; 4];
        rdr.read_exact(&mut buf)
            .map_err(|_| DecodeError::ShortInput)?;
        let id = u32::from_le_bytes(buf);
        table.push(id);
        Ok((id, &input[4..]))
    } else {
        let idx = tag as usize;
        if idx >= table.len() {
            return Err(if is_uid {
                DecodeError::BadUid(tag)
            } else {
                DecodeError::BadGid(tag)
            });
        }
        Ok((table[idx], input))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_paths_and_ids() {
        let entries = vec![
            Entry {
                path: "dir/file1".into(),
                uid: 1000,
                gid: 1000,
            },
            Entry {
                path: "dir/file2".into(),
                uid: 1000,
                gid: 1001,
            },
            Entry {
                path: "other".into(),
                uid: 1002,
                gid: 1001,
            },
        ];
        let mut enc = Encoder::new();
        let mut dec = Decoder::new();
        for e in entries {
            let bytes = enc.encode_entry(&e);
            let d = dec.decode_entry(&bytes).unwrap();
            assert_eq!(d, e);
        }
    }
}
