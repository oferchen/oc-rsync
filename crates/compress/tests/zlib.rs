// crates/compress/tests/zlib.rs
#[cfg(feature = "zlib")]
use compress::{Compressor, Zlib};

#[cfg(feature = "zlib")]
#[test]
fn zlib_new_clamps_level() {
    let data = b"clamp test data";
    let compress_with = |lvl: i32| {
        let codec = Zlib::new(lvl);
        let mut compressed = Vec::new();
        codec
            .compress(&mut &data[..], &mut compressed)
            .expect("compress");
        compressed
    };

    assert_eq!(compress_with(10), compress_with(9));
    assert_eq!(compress_with(-5), compress_with(0));
}
