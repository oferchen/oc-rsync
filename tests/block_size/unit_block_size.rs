// tests/block_size/unit_block_size.rs
use engine::block_size;
use std::fs;

fn expected(len: u64) -> usize {
    let data = fs::read_to_string("tests/golden/block_size/upstream_block_sizes.txt").unwrap();
    for line in data.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let mut parts = line.split_whitespace();
        let l: u64 = parts.next().unwrap().parse().unwrap();
        if l == len {
            return parts.next().unwrap().parse().unwrap();
        }
    }
    panic!("length {len} not found");
}

#[test]
fn len_100() {
    assert_eq!(block_size(100), expected(100));
}

#[test]
fn len_490000() {
    assert_eq!(block_size(490_000), expected(490_000));
}

#[test]
fn len_500000() {
    assert_eq!(block_size(500_000), expected(500_000));
}

#[test]
fn len_1048576() {
    assert_eq!(block_size(1_048_576), expected(1_048_576));
}

#[test]
fn len_10000000() {
    assert_eq!(block_size(10_000_000), expected(10_000_000));
}

#[test]
fn len_100000000() {
    assert_eq!(block_size(100_000_000), expected(100_000_000));
}

#[test]
fn len_1000000000() {
    assert_eq!(block_size(1_000_000_000), expected(1_000_000_000));
}

#[test]
fn len_1000000000000() {
    assert_eq!(block_size(1_000_000_000_000), expected(1_000_000_000_000));
}
