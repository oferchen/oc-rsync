// crates/checksums/benches/rolling.rs
#[cfg(feature = "nightly")]
use checksums::rolling_checksum_avx512;
use checksums::{rolling_checksum_avx2, rolling_checksum_scalar, rolling_checksum_sse42};
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_rolling(c: &mut Criterion) {
    let data = vec![0u8; 1024 * 1024];
    c.bench_function("scalar", |b| {
        b.iter(|| black_box(rolling_checksum_scalar(&data, 0)));
    });
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    {
        if std::arch::is_x86_feature_detected!("sse4.2") {
            c.bench_function("sse4.2", |b| {
                b.iter(|| black_box(unsafe { rolling_checksum_sse42(&data, 0) }));
            });
        }
        if std::arch::is_x86_feature_detected!("avx2") {
            c.bench_function("avx2", |b| {
                b.iter(|| black_box(unsafe { rolling_checksum_avx2(&data, 0) }));
            });
        }
        #[cfg(feature = "nightly")]
        if std::arch::is_x86_feature_detected!("avx512f") {
            c.bench_function("avx512", |b| {
                b.iter(|| black_box(unsafe { rolling_checksum_avx512(&data, 0) }));
            });
        }
    }
}

criterion_group!(benches, bench_rolling);
criterion_main!(benches);
