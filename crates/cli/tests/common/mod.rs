// crates/cli/tests/common/mod.rs
#![allow(dead_code)]

pub fn parse_literal(stats: &str) -> usize {
    for line in stats.lines() {
        let line = line.trim();
        if let Some(rest) = line
            .strip_prefix("Literal data: ")
            .or_else(|| line.strip_prefix("Unmatched data: "))
        {
            let num_str = rest.split_whitespace().next().unwrap().replace(",", "");
            return num_str.parse().unwrap();
        }
    }
    panic!("no literal data in stats: {stats}");
}
