// crates/filelist/src/entry.rs

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub path: Vec<u8>,
    pub uid: u32,
    pub gid: u32,
    pub hardlink: Option<u32>,
    pub xattrs: Vec<(Vec<u8>, Vec<u8>)>,
    pub acl: Vec<u8>,
    pub default_acl: Vec<u8>,
}

#[cfg(unix)]
#[derive(Debug, Clone)]
pub struct InodeEntry {
    pub path: Vec<u8>,
    pub uid: u32,
    pub gid: u32,
    pub dev: u64,
    pub ino: u64,
    pub xattrs: Vec<(Vec<u8>, Vec<u8>)>,
    pub acl: Vec<u8>,
    pub default_acl: Vec<u8>,
}

#[cfg(unix)]
pub fn group_by_inode(entries: &[InodeEntry]) -> Vec<Entry> {
    use meta::hard_link_id;
    let mut counts: HashMap<u64, usize> = HashMap::new();
    for e in entries {
        let id = hard_link_id(e.dev, e.ino);
        *counts.entry(id).or_default() += 1;
    }
    let mut groups: HashMap<u64, u32> = HashMap::new();
    let mut next: u32 = 0;
    let mut out = Vec::with_capacity(entries.len());
    for e in entries {
        let id = hard_link_id(e.dev, e.ino);
        let hardlink = if counts.get(&id).copied().unwrap_or(0) > 1 {
            Some(*groups.entry(id).or_insert_with(|| {
                let g = next;
                next += 1;
                g
            }))
        } else {
            None
        };
        out.push(Entry {
            path: e.path.clone(),
            uid: e.uid,
            gid: e.gid,
            hardlink,
            xattrs: e.xattrs.clone(),
            acl: e.acl.clone(),
            default_acl: e.default_acl.clone(),
        });
    }
    out
}
