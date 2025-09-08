// crates/engine/src/block.rs

const RSYNC_BLOCK_SIZE: usize = 700;
const RSYNC_MAX_BLOCK_SIZE: usize = 1 << 17;

pub fn block_size(len: u64) -> usize {
    if len <= (RSYNC_BLOCK_SIZE * RSYNC_BLOCK_SIZE) as u64 {
        return RSYNC_BLOCK_SIZE;
    }
    let mut c: usize = 1;
    let mut l = len;
    while (l >> 2) != 0 {
        l >>= 2;
        c <<= 1;
    }
    if c >= RSYNC_MAX_BLOCK_SIZE {
        return RSYNC_MAX_BLOCK_SIZE;
    }
    let mut blength = 0usize;
    while c >= 8 {
        blength |= c;
        if len < (blength as u64) * (blength as u64) {
            blength &= !c;
        }
        c >>= 1;
    }
    blength.max(RSYNC_BLOCK_SIZE)
}
