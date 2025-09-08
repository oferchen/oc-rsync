// crates/filelist/src/encoder.rs

use std::collections::HashMap;

use crate::entry::Entry;

#[derive(Debug, Default)]
pub struct Encoder {
    prev_path: Vec<u8>,
    uid_table: HashMap<u32, u8>,
    gid_table: HashMap<u32, u8>,
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
        if let Some(group) = entry.hardlink {
            out.push(1);
            out.extend_from_slice(&encode_id(group, &mut self.gid_table));
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
