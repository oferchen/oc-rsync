// crates/engine/tests/receiver.rs
use engine::{Op, Receiver, SyncOptions};
use std::fs;
use std::path::Path;
use tempfile::tempdir;

#[test]
#[allow(clippy::field_reassign_with_default)]
fn apply_without_existing_partial() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src.txt");
    let dest = tmp.path().join("dest.txt");
    fs::write(&src, b"hello").unwrap();

    let mut opts = SyncOptions::default();
    opts.partial = true;
    let mut recv = Receiver::new(None, opts);

    let delta = vec![Ok(Op::Data(b"hello".to_vec()))];
    recv.apply(&src, &dest, Path::new(""), delta).unwrap();

    let output = fs::read(&dest).unwrap();
    assert_eq!(output, b"hello");
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn apply_with_short_partial() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src.txt");
    let dest = tmp.path().join("dest.txt");
    fs::write(&src, b"hello").unwrap();
    let mut name = dest.file_name().unwrap().to_os_string();
    name.push(".partial");
    let partial = dest.with_file_name(name);
    fs::write(&partial, b"he").unwrap();

    let mut opts = SyncOptions::default();
    opts.partial = true;
    let mut recv = Receiver::new(None, opts);

    let delta = vec![Ok(Op::Data(b"hello".to_vec()))];
    recv.apply(&src, &dest, Path::new(""), delta).unwrap();

    let output = fs::read(&dest).unwrap();
    assert_eq!(output, b"hello");
    assert!(!partial.exists());
}

#[test]
#[allow(clippy::field_reassign_with_default)]
fn apply_with_existing_partial() {
    let tmp = tempdir().unwrap();
    let src = tmp.path().join("src.txt");
    let dest = tmp.path().join("dest.txt");
    fs::write(&src, b"old!").unwrap();
    let mut name = dest.file_name().unwrap().to_os_string();
    name.push(".partial");
    let partial = dest.with_file_name(name);
    fs::write(&partial, b"old").unwrap();

    let mut opts = SyncOptions::default();
    opts.partial = true;
    let mut recv = Receiver::new(None, opts);

    let delta = vec![
        Ok(Op::Copy { offset: 0, len: 3 }),
        Ok(Op::Data(b"!".to_vec())),
    ];
    recv.apply(&src, &dest, Path::new(""), delta).unwrap();

    let output = fs::read(&dest).unwrap();
    assert_eq!(output, b"old!");
    assert!(!partial.exists());
}
