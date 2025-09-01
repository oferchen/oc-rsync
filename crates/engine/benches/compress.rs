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
        let compressed = zstd.compress(&data).unwrap();
        c.bench_function("zstd_compress_1mb", |b| {
            b.iter(|| {
                zstd.compress(&data).unwrap();
            });
        });
        c.bench_function("zstd_decompress_1mb", |b| {
            b.iter(|| {
                zstd.decompress(&compressed).unwrap();
            });
        });
    }
}

criterion_group!(benches, bench_compress);
criterion_main!(benches);
