// xtask/src/bin/status.rs
use serde_json::Value;
use std::fs;

fn read_coverage() -> f64 {
    let text = fs::read_to_string("reports/coverage.json").unwrap_or_default();
    let v: Value = serde_json::from_str(&text).unwrap_or_default();
    v.pointer("/data/0/totals/lines/percent")
        .and_then(|x| x.as_f64())
        .unwrap_or(0.0)
}

fn count_features() -> (usize, usize) {
    let text = fs::read_to_string("docs/feature_matrix.md").unwrap_or_default();
    let mut total = 0;
    let mut done = 0;
    for line in text.lines() {
        if line.starts_with("| `--") {
            total += 1;
            if line.contains("| ✅ |") {
                done += 1;
            }
        }
    }
    (done, total)
}

fn main() {
    let cov = read_coverage();
    let (done, total) = count_features();
    let out = format!("# Daily Estimate\n\ncoverage: {cov:.1}%\nfeatures: {done}/{total}\n");
    fs::create_dir_all("reports").ok();
    fs::write("reports/daily_estimate.md", out).unwrap();
}
