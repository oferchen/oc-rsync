// crates/engine/src/batch.rs

use std::fs;
use std::path::{Path, PathBuf};

use crate::{EngineError, Result};

fn unescape_rsync(path: &str) -> String {
    let mut bytes = Vec::with_capacity(path.len());
    let mut iter = path.bytes();
    while let Some(b) = iter.next() {
        if b == b'\\' {
            let oct: Vec<u8> = iter.clone().take(3).collect();
            if oct.len() == 3 && oct.iter().all(|c| c.is_ascii_digit()) {
                let val = (oct[0] - b'0') * 64 + (oct[1] - b'0') * 8 + (oct[2] - b'0');
                bytes.push(val);
                iter.nth(2);
                continue;
            }
        }
        bytes.push(b);
    }
    String::from_utf8(bytes).unwrap_or_else(|_| path.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Batch {
    pub flist: Vec<Vec<u8>>,
    pub checksums: Vec<Vec<u8>>,
    pub data: Vec<Vec<u8>>,
}

fn encode_section(out: &mut Vec<u8>, parts: &[Vec<u8>]) {
    out.extend((parts.len() as u32).to_le_bytes());
    for part in parts {
        out.extend((part.len() as u32).to_le_bytes());
        out.extend(part);
    }
}

pub fn encode_batch(batch: &Batch) -> Vec<u8> {
    let mut out = Vec::new();
    encode_section(&mut out, &batch.flist);
    encode_section(&mut out, &batch.checksums);
    encode_section(&mut out, &batch.data);
    out
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32> {
    if *pos + 4 > bytes.len() {
        return Err(EngineError::Other("truncated batch".into()));
    }
    let mut arr = [0u8; 4];
    arr.copy_from_slice(&bytes[*pos..*pos + 4]);
    *pos += 4;
    Ok(u32::from_le_bytes(arr))
}

fn decode_section(bytes: &[u8], pos: &mut usize) -> Result<Vec<Vec<u8>>> {
    let count = read_u32(bytes, pos)? as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let len = read_u32(bytes, pos)? as usize;
        if *pos + len > bytes.len() {
            return Err(EngineError::Other("truncated batch".into()));
        }
        out.push(bytes[*pos..*pos + len].to_vec());
        *pos += len;
    }
    Ok(out)
}

pub fn decode_batch(bytes: &[u8]) -> Result<Batch> {
    let mut pos = 0;
    let flist = decode_section(bytes, &mut pos)?;
    let checksums = decode_section(bytes, &mut pos)?;
    let data = decode_section(bytes, &mut pos)?;
    Ok(Batch {
        flist,
        checksums,
        data,
    })
}

pub(crate) fn parse_batch_file(batch_path: &Path) -> Result<Vec<PathBuf>> {
    let content = fs::read_to_string(batch_path).map_err(|e| EngineError::Other(e.to_string()))?;
    let mut paths = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.contains('=') {
            continue;
        }
        paths.push(PathBuf::from(unescape_rsync(trimmed)));
    }
    Ok(paths)
}
