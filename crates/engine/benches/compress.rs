// crates/engine/benches/compress.rs
#[cfg(feature = "lz4")]
use compress::Lz4;
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
    #[cfg(feature = "lz4")]
    {
        let lz4 = Lz4;
        let compressed = lz4.compress(&data).unwrap();
        c.bench_function("lz4_compress_1mb", |b| {
            b.iter(|| {
                lz4.compress(&data).unwrap();
            });
        });
        c.bench_function("lz4_decompress_1mb", |b| {
            b.iter(|| {
                lz4.decompress(&compressed).unwrap();
            });
        });
    }
}

criterion_group!(benches, bench_compress);
criterion_main!(benches);
