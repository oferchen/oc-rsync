// crates/engine/benches/compress.rs
#[cfg(feature = "zstd")]
use compress::Zstd;
use compress::{Compressor, Decompressor};
use criterion::{criterion_group, criterion_main, Criterion};

fn bench_compress(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024];
    #[cfg(feature = "zstd")]
    {
        let zstd = Zstd::default();
        let mut compressed = Vec::new();
        let mut src = data.as_slice();
        zstd.compress(&mut src, &mut compressed).unwrap();
        c.bench_function("zstd_compress_1mb", |b| {
            b.iter(|| {
                let mut out = Vec::new();
                let mut cursor = data.as_slice();
                zstd.compress(&mut cursor, &mut out).unwrap();
            });
        });
        c.bench_function("zstd_decompress_1mb", |b| {
            b.iter(|| {
                let mut out = Vec::new();
                let mut cursor = compressed.as_slice();
                zstd.decompress(&mut cursor, &mut out).unwrap();
            });
        });
    }
}

criterion_group!(benches, bench_compress);
criterion_main!(benches);
