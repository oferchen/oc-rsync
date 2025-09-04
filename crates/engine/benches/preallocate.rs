// crates/engine/benches/preallocate.rs
use compress::available_codecs;
use criterion::{criterion_group, criterion_main, Criterion};
use engine::{sync, SyncOptions};
use filters::Matcher;
use std::fs;
use tempfile::tempdir;

fn bench_preallocate(c: &mut Criterion) {
    c.bench_function("preallocate_10mb", |b| {
        b.iter(|| {
            let dir = tempdir().unwrap();
            let src = dir.path().join("src");
            let dst = dir.path().join("dst");
            fs::create_dir_all(&src).unwrap();
            fs::create_dir_all(&dst).unwrap();
            fs::write(src.join("file.bin"), vec![0u8; 10 * 1024 * 1024]).unwrap();
            let opts = SyncOptions {
                preallocate: true,
                ..SyncOptions::default()
            };
            sync(&src, &dst, &Matcher::default(), &available_codecs(), &opts).unwrap();
        });
    });
}

criterion_group!(benches, bench_preallocate);
criterion_main!(benches);
