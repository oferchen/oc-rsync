// crates/filelist/src/lib.rs

use std::collections::HashMap;
use std::io::Read;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: Vec<u8>,
    pub uid: u32,
    pub gid: u32,
    pub group: Option<u32>,
    pub xattrs: Vec<(Vec<u8>, Vec<u8>)>,
    pub acl: Vec<u8>,
    pub default_acl: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct Encoder {
    prev_path: Vec<u8>,
    uid_table: HashMap<u32, u8>,
    gid_table: HashMap<u32, u8>,
}

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
        out.extend_from_slice(suffix);
        out.extend_from_slice(&encode_id(entry.uid, &mut self.uid_table));
        out.extend_from_slice(&encode_id(entry.gid, &mut self.gid_table));
        if let Some(group) = entry.group {
            out.push(1);
            out.extend_from_slice(&group.to_le_bytes());
        } else {
            out.push(0);
        }
        out.push(entry.xattrs.len() as u8);
        for (name, value) in &entry.xattrs {
            out.push(name.len() as u8);
            out.extend_from_slice(name);
            out.extend_from_slice(&(value.len() as u32).to_le_bytes());
            out.extend_from_slice(value);
        }
        out.extend_from_slice(&(entry.acl.len() as u32).to_le_bytes());
        out.extend_from_slice(&entry.acl);
        out.extend_from_slice(&(entry.default_acl.len() as u32).to_le_bytes());
        out.extend_from_slice(&entry.default_acl);
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
        let path: Vec<u8> = self.prev_path[..common]
            .iter()
            .copied()
            .chain(suffix.iter().copied())
            .collect();
        let (uid, rest) = decode_id(input, &mut self.uid_table, true)?;
        let (gid, mut rest) = decode_id(rest, &mut self.gid_table, false)?;
        let group = if rest.is_empty() {
            return Err(DecodeError::ShortInput);
        } else {
            let tag = rest[0];
            rest = &rest[1..];
            if tag == 1 {
                if rest.len() < 4 {
                    return Err(DecodeError::ShortInput);
                }
                let mut buf = [0u8; 4];
                buf.copy_from_slice(&rest[..4]);
                rest = &rest[4..];
                Some(u32::from_le_bytes(buf))
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
            group,
            xattrs,
            acl,
            default_acl,
        })
    }
}

fn common_prefix(a: &[u8], b: &[u8]) -> usize {
    a.iter().zip(b.iter()).take_while(|(x, y)| x == y).count()
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
                path: b"dir/file1".to_vec(),
                uid: 1000,
                gid: 1000,
                group: None,
                xattrs: vec![(b"user.test".to_vec(), b"val".to_vec())],
                acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
                default_acl: Vec::new(),
            },
            Entry {
                path: b"dir/file2".to_vec(),
                uid: 1000,
                gid: 1001,
                group: None,
                xattrs: Vec::new(),
                acl: Vec::new(),
                default_acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
            },
            Entry {
                path: b"other".to_vec(),
                uid: 1002,
                gid: 1001,
                group: None,
                xattrs: Vec::new(),
                acl: Vec::new(),
                default_acl: Vec::new(),
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
