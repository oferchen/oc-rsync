// fuzz/src/lib.rs
//! Fuzzing helpers for oc-rsync.
#![deny(unsafe_op_in_unsafe_fn, rust_2018_idioms)]
#![deny(warnings)]
#![warn(missing_docs)]

pub mod helpers {
    use std::io::Cursor;

    #[inline]
    pub fn cursor(data: &[u8]) -> Cursor<&[u8]> {
        Cursor::new(data)
    }

    #[inline]
    pub fn as_str(data: &[u8]) -> Option<&str> {
        std::str::from_utf8(data).ok()
    }
}
