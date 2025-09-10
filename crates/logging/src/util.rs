// crates/logging/src/util.rs
#![allow(missing_docs)]

use std::path::Path;
use time::{OffsetDateTime, macros::format_description};

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 9] = ["", "K", "M", "G", "T", "P", "E", "Z", "Y"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        size.trunc().to_string()
    } else {
        format!("{:.2}{}", size, UNITS[unit])
    }
}

pub fn progress_formatter(bytes: u64, human_readable: bool) -> String {
    if human_readable {
        human_bytes(bytes)
    } else {
        let mut n = bytes;
        let mut parts = Vec::new();
        while n >= 1000 {
            parts.push(format!("{:03}", n % 1000));
            n /= 1000;
        }
        parts.push(n.to_string());
        parts.reverse();
        parts.join(",")
    }
}

pub fn rate_formatter(bytes_per_sec: f64) -> String {
    let mut rate = bytes_per_sec / 1024.0;
    let mut units = "KB/s";
    if rate > 1024.0 {
        rate /= 1024.0;
        units = "MB/s";
        if rate > 1024.0 {
            rate /= 1024.0;
            units = "GB/s";
        }
    }
    format!("{:>7.2}{}", rate, units)
}

fn escape_bytes(bytes: &[u8], eight_bit_output: bool) -> String {
    let mut out = String::new();
    for &b in bytes {
        if (b < 0x20 && b != b'\t') || (!eight_bit_output && b > 0x7e) {
            out.push_str(&format!("\\#{:03o}", b));
        } else {
            out.push(char::from(b));
        }
    }
    out
}

pub fn escape_path(path: &Path, eight_bit_output: bool) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        escape_bytes(path.as_os_str().as_bytes(), eight_bit_output)
    }
    #[cfg(not(unix))]
    {
        escape_bytes(path.to_string_lossy().as_bytes(), eight_bit_output)
    }
}

pub fn parse_escapes(input: &str) -> String {
    let mut out = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('a') => out.push('\x07'),
                Some('b') => out.push('\x08'),
                Some('e') => out.push('\x1b'),
                Some('f') => out.push('\x0c'),
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('v') => out.push('\x0b'),
                Some('x') => {
                    let mut val = 0u32;
                    let mut digits = 0;
                    for _ in 0..2 {
                        if let Some(peek) = chars.peek().copied() {
                            if let Some(digit) = peek.to_digit(16) {
                                val = (val << 4) + digit;
                                chars.next();
                                digits += 1;
                            } else {
                                break;
                            }
                        }
                    }
                    if digits == 0 {
                        tracing::warn!("invalid hex escape sequence");
                        out.push('x');
                    } else if let Some(ch) = char::from_u32(val) {
                        out.push(ch);
                    } else {
                        tracing::warn!("invalid hex escape value: {val}");
                    }
                }
                Some('\\') => out.push('\\'),
                Some(c @ '0'..='7') => {
                    let mut val = match c.to_digit(8) {
                        Some(d) => d,
                        None => {
                            tracing::warn!("invalid octal escape sequence");
                            out.push(c);
                            continue;
                        }
                    };
                    for _ in 0..2 {
                        if let Some(peek) = chars.peek().copied() {
                            if ('0'..='7').contains(&peek) {
                                if let Some(digit) = peek.to_digit(8) {
                                    val = (val << 3) + digit;
                                    chars.next();
                                } else {
                                    break;
                                }
                            } else {
                                break;
                            }
                        }
                    }
                    if let Some(ch) = char::from_u32(val) {
                        out.push(ch);
                    } else {
                        tracing::warn!("invalid octal escape value: {val}");
                    }
                }
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub struct OutFormatOptions<'a> {
    name: &'a Path,
    link: Option<&'a Path>,
    itemized: Option<&'a str>,
    eight_bit_output: bool,
}

impl<'a> OutFormatOptions<'a> {
    pub fn new(name: &'a Path) -> Self {
        Self {
            name,
            link: None,
            itemized: None,
            eight_bit_output: false,
        }
    }

    pub fn link(mut self, link: Option<&'a Path>) -> Self {
        self.link = link;
        self
    }

    pub fn itemized(mut self, itemized: Option<&'a str>) -> Self {
        self.itemized = itemized;
        self
    }

    pub fn eight_bit_output(mut self, eight_bit_output: bool) -> Self {
        self.eight_bit_output = eight_bit_output;
        self
    }
}

pub fn render_out_format(format: &str, opts: &OutFormatOptions<'_>) -> String {
    let fmt = parse_escapes(format);
    let mut out = String::new();
    let mut chars = fmt.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            if let Some(n) = chars.next() {
                match n {
                    'n' | 'f' => out.push_str(&escape_path(opts.name, opts.eight_bit_output)),
                    'L' => {
                        if let Some(l) = opts.link {
                            out.push_str(" -> ");
                            out.push_str(&escape_path(l, opts.eight_bit_output));
                        }
                    }
                    'i' => {
                        if let Some(i) = opts.itemized {
                            out.push_str(i);
                        }
                    }
                    'o' => {
                        if let Some(i) = opts.itemized {
                            let op = match i.chars().next().unwrap_or(' ') {
                                '>' => "send",
                                '<' => "recv",
                                '*' => "del.",
                                _ => "",
                            };
                            out.push_str(op);
                        }
                    }
                    'p' => out.push_str(&std::process::id().to_string()),
                    't' => {
                        let now = OffsetDateTime::now_local()
                            .unwrap_or_else(|_| OffsetDateTime::now_utc());
                        let fmt =
                            format_description!("[year]/[month]/[day] [hour]:[minute]:[second]");
                        let ts = now
                            .format(&fmt)
                            .unwrap_or_else(|_| String::from("0000/00/00 00:00:00"));
                        out.push_str(&ts);
                    }
                    '%' => out.push('%'),
                    other => {
                        out.push('%');
                        out.push(other);
                    }
                }
            } else {
                out.push('%');
            }
        } else {
            out.push(c);
        }
    }
    out
}
