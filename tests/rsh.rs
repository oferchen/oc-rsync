use std::fs;
use tempfile::tempdir;
#[cfg(unix)]
mod remote_utils;
#[cfg(unix)]
use remote_utils::{spawn_reader, spawn_writer};

#[cfg(unix)]
#[test]
fn rsh_remote_pair_syncs() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    let dst = dir.path().join("dst.txt");
    fs::write(&src, b"via rsh").unwrap();

    let src_session = spawn_reader(&format!("cat {}", src.display()));
    let dst_session = spawn_writer(&format!("cat > {}", dst.display()));
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    std::io::copy(&mut src_reader, &mut dst_writer).unwrap();
    drop(dst_writer);
    drop(src_reader);
    std::thread::sleep(std::time::Duration::from_millis(50));

    let out = fs::read(&dst).unwrap();
    assert_eq!(out, b"via rsh");
}
