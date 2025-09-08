// crates/filelist/src/decoder.rs

use std::io::Read;

use thiserror::Error;

use crate::entry::Entry;

#[derive(Debug, Default)]
pub struct Decoder {
    prev_path: Vec<u8>,
    uid_table: Vec<u32>,
    gid_table: Vec<u32>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("input too short")]
    ShortInput,
    #[error("unknown uid index {0}")]
    BadUid(u8),
    #[error("unknown gid index {0}")]
    BadGid(u8),
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
        let path: Vec<u8> = self.prev_path[..common]
            .iter()
            .copied()
            .chain(suffix.iter().copied())
            .collect();
        let (uid, rest) = decode_id(input, &mut self.uid_table, true)?;
        let (gid, mut rest) = decode_id(rest, &mut self.gid_table, false)?;
        let hardlink = if rest.is_empty() {
            return Err(DecodeError::ShortInput);
        } else {
            let tag = rest[0];
            rest = &rest[1..];
            if tag == 1 {
                let (g, r) = decode_id(rest, &mut self.gid_table, false)?;
                rest = r;
                Some(g)
            } else {
                None
            }
        };
        if rest.is_empty() {
            return Err(DecodeError::ShortInput);
        }
        let xcnt = rest[0] as usize;
        rest = &rest[1..];
        let mut xattrs = Vec::new();
        for _ in 0..xcnt {
            if rest.is_empty() {
                return Err(DecodeError::ShortInput);
            }
            let nlen = rest[0] as usize;
            rest = &rest[1..];
            if rest.len() < nlen {
                return Err(DecodeError::ShortInput);
            }
            let name = rest[..nlen].to_vec();
            rest = &rest[nlen..];
            if rest.len() < 4 {
                return Err(DecodeError::ShortInput);
            }
            let vlen = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
            rest = &rest[4..];
            if rest.len() < vlen {
                return Err(DecodeError::ShortInput);
            }
            let value = rest[..vlen].to_vec();
            rest = &rest[vlen..];
            xattrs.push((name, value));
        }
        if rest.len() < 4 {
            return Err(DecodeError::ShortInput);
        }
        let acl_len = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        rest = &rest[4..];
        if rest.len() < acl_len {
            return Err(DecodeError::ShortInput);
        }
        let acl = rest[..acl_len].to_vec();
        rest = &rest[acl_len..];
        if rest.len() < 4 {
            return Err(DecodeError::ShortInput);
        }
        let dacl_len = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
        rest = &rest[4..];
        if rest.len() < dacl_len {
            return Err(DecodeError::ShortInput);
        }
        let default_acl = rest[..dacl_len].to_vec();
        rest = &rest[dacl_len..];
        debug_assert!(rest.is_empty());
        self.prev_path = path.clone();
        Ok(Entry {
            path,
            uid,
            gid,
            hardlink,
            xattrs,
            acl,
            default_acl,
        })
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
