use tempfile::tempdir;
#[cfg(unix)]
mod remote_utils;
#[cfg(unix)]
use remote_utils::{spawn_reader, spawn_writer};
use std::fs;

#[cfg(unix)]
#[test]
fn server_remote_pair_reports_error() {
    let dir = tempdir().unwrap();
    let src = dir.path().join("src.txt");
    fs::write(&src, b"data").unwrap();

    let src_session = spawn_reader(&format!("cat {}", src.display()));
    let dst_session = spawn_writer("exec 0<&-; sleep 1");
    let (mut src_reader, _) = src_session.into_inner();
    let (_, mut dst_writer) = dst_session.into_inner();
    let res = std::io::copy(&mut src_reader, &mut dst_writer);
    assert!(res.is_err());
}
