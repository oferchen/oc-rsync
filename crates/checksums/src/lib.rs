use md5::{Digest, Md5};

/// Compute the rsync rolling checksum for a block of data.
pub fn rolling_checksum(data: &[u8]) -> u32 {
    let mut s1: u32 = 0;
    let mut s2: u32 = 0;
    let n = data.len();
    for (i, b) in data.iter().enumerate() {
        s1 = s1.wrapping_add(*b as u32);
        s2 = s2.wrapping_add((n - i) as u32 * (*b as u32));
    }
    (s1 & 0xffff) | (s2 << 16)
}

/// Rolling checksum state allowing incremental updates.
#[derive(Debug, Clone)]
pub struct Rolling {
    len: usize,
    s1: u32,
    s2: u32,
}

impl Rolling {
    pub fn new(block: &[u8]) -> Self {
        let mut r = Rolling {
            len: block.len(),
            s1: 0,
            s2: 0,
        };
        for (i, b) in block.iter().enumerate() {
            r.s1 = r.s1.wrapping_add(*b as u32);
            r.s2 = r.s2.wrapping_add((block.len() - i) as u32 * (*b as u32));
        }
        r
    }

    pub fn roll(&mut self, out: u8, inp: u8) {
        self.s1 = self.s1.wrapping_sub(out as u32).wrapping_add(inp as u32);
        self.s2 = self
            .s2
            .wrapping_sub(self.len as u32 * out as u32)
            .wrapping_add(self.s1);
    }

    pub fn digest(&self) -> u32 {
        (self.s1 & 0xffff) | (self.s2 << 16)
    }
}

/// Compute the strong MD5 digest for data.
pub fn strong_digest(data: &[u8]) -> [u8; 16] {
    let mut hasher = Md5::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut out = [0u8; 16];
    out.copy_from_slice(&result);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rolling_known() {
        let sum = rolling_checksum(b"hello world");
        assert_eq!(sum, 436208732); // verified against rsync implementation
    }

    #[test]
    fn rolling_slide() {
        let mut r = Rolling::new(b"hello w");
        r.roll(b'h', b'!');
        assert_eq!(r.digest(), rolling_checksum(b"ello w!"));
    }

    #[test]
    fn strong_md5() {
        let digest = strong_digest(b"hello world");
        assert_eq!(hex::encode(digest), "5eb63bbbe01eeed093cb22bb8f5acdc3");
    }
}
