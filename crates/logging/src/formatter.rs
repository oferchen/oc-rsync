// crates/logging/src/formatter.rs
use crate::parse_escapes;
use std::collections::HashMap;
use std::fmt;
use time::{macros::format_description, OffsetDateTime};
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct RsyncFormatter {
    tokens: Option<Vec<Token>>,
}

impl RsyncFormatter {
    pub fn new(format: Option<String>) -> Self {
        let tokens = format.map(|f| {
            let fmt = parse_escapes(&f);
            parse_tokens(&fmt)
        });
        Self { tokens }
    }

    fn columns() -> usize {
        std::env::var("COLUMNS")
            .ok()
            .and_then(|v| v.parse().ok())
            .filter(|&c| c > 0)
            .unwrap_or(80)
    }

    fn wrap(msg: &str, width: usize) -> String {
        let mut out = String::new();
        let mut line_len = 0usize;
        for word in msg.split_whitespace() {
            let wlen = word.len();
            if line_len == 0 {
                out.push_str(word);
                line_len = wlen;
            } else if line_len + 1 + wlen > width {
                out.push('\n');
                out.push_str(word);
                line_len = wlen;
            } else {
                out.push(' ');
                out.push_str(word);
                line_len += 1 + wlen;
            }
        }
        out
    }
}

#[derive(Clone)]
enum Token {
    Lit(String),
    Percent,
    Addr,
    Bytes,
    Perms,
    Checksum,
    FullChecksum,
    File,
    Gid,
    Host,
    Itemized,
    Length,
    Symlink,
    Module,
    ModTime,
    Name,
    Operation,
    Pid,
    ModulePath,
    Time,
    User,
    Uid,
}

fn parse_tokens(fmt: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = fmt.chars();
    let mut lit = String::new();
    while let Some(c) = chars.next() {
        if c == '%' {
            if !lit.is_empty() {
                tokens.push(Token::Lit(std::mem::take(&mut lit)));
            }
            match chars.next() {
                Some('%') => tokens.push(Token::Percent),
                Some('a') => tokens.push(Token::Addr),
                Some('b') => tokens.push(Token::Bytes),
                Some('B') => tokens.push(Token::Perms),
                Some('c') => tokens.push(Token::Checksum),
                Some('C') => tokens.push(Token::FullChecksum),
                Some('f') => tokens.push(Token::File),
                Some('G') => tokens.push(Token::Gid),
                Some('h') => tokens.push(Token::Host),
                Some('i') => tokens.push(Token::Itemized),
                Some('l') => tokens.push(Token::Length),
                Some('L') => tokens.push(Token::Symlink),
                Some('m') => tokens.push(Token::Module),
                Some('M') => tokens.push(Token::ModTime),
                Some('n') => tokens.push(Token::Name),
                Some('o') => tokens.push(Token::Operation),
                Some('p') => tokens.push(Token::Pid),
                Some('P') => tokens.push(Token::ModulePath),
                Some('t') => tokens.push(Token::Time),
                Some('u') => tokens.push(Token::User),
                Some('U') => tokens.push(Token::Uid),
                Some(other) => {
                    lit.push('%');
                    lit.push(other);
                }
                None => lit.push('%'),
            }
        } else {
            lit.push(c);
        }
    }
    if !lit.is_empty() {
        tokens.push(Token::Lit(lit));
    }
    tokens
}

struct MsgVisitor {
    msg: String,
    fields: HashMap<String, String>,
}

impl MsgVisitor {
    fn new() -> Self {
        Self {
            msg: String::new(),
            fields: HashMap::new(),
        }
    }
}

impl Visit for MsgVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(value);
        } else {
            self.fields
                .insert(field.name().to_string(), value.to_string());
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(&format!("{value:?}"));
        } else {
            self.fields
                .insert(field.name().to_string(), format!("{value:?}"));
        }
    }
}

fn format_time() -> String {
    let now = OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc());
    let fmt = format_description!("[year]/[month]/[day] [hour]:[minute]:[second]");
    now.format(&fmt).unwrap()
}

impl<S, N> FormatEvent<S, N> for RsyncFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let mut visitor = MsgVisitor::new();
        event.record(&mut visitor);
        if let Some(tokens) = &self.tokens {
            let mut out = format!("{} [{}] ", format_time(), std::process::id());
            for tok in tokens {
                match tok {
                    Token::Lit(s) => out.push_str(s),
                    Token::Percent => out.push('%'),
                    Token::Time => out.push_str(&format_time()),
                    Token::Pid => out.push_str(&std::process::id().to_string()),
                    Token::Addr => {
                        if let Some(v) = visitor.fields.get("addr") {
                            out.push_str(v);
                        }
                    }
                    Token::Bytes => {
                        if let Some(v) = visitor.fields.get("bytes") {
                            out.push_str(v);
                        }
                    }
                    Token::Perms => {
                        if let Some(v) = visitor.fields.get("perms") {
                            out.push_str(v);
                        }
                    }
                    Token::Checksum => {
                        if let Some(v) = visitor.fields.get("checksum") {
                            out.push_str(v);
                        }
                    }
                    Token::FullChecksum => {
                        if let Some(v) = visitor.fields.get("full_checksum") {
                            out.push_str(v);
                        }
                    }
                    Token::File | Token::Name => {
                        if let Some(v) = visitor
                            .fields
                            .get("file")
                            .or_else(|| visitor.fields.get("name"))
                            .or_else(|| visitor.fields.get("path"))
                        {
                            out.push_str(v);
                        } else if let Some(f) = visitor.msg.split_whitespace().nth(1) {
                            out.push_str(f);
                        }
                    }
                    Token::Gid => {
                        if let Some(v) = visitor.fields.get("gid") {
                            out.push_str(v);
                        }
                    }
                    Token::Host => {
                        if let Some(v) = visitor.fields.get("host") {
                            out.push_str(v);
                        }
                    }
                    Token::Itemized => {
                        if let Some(v) = visitor.fields.get("itemized") {
                            out.push_str(v);
                        } else if let Some(i) = visitor.msg.split_whitespace().next() {
                            out.push_str(i);
                        }
                    }
                    Token::Length => {
                        if let Some(v) = visitor.fields.get("len") {
                            out.push_str(v);
                        }
                    }
                    Token::Symlink => {
                        if let Some(v) = visitor.fields.get("link") {
                            out.push_str(" -> ");
                            out.push_str(v);
                        }
                    }
                    Token::Module => {
                        if let Some(v) = visitor.fields.get("module") {
                            out.push_str(v);
                        }
                    }
                    Token::ModTime => {
                        if let Some(v) = visitor.fields.get("mtime") {
                            out.push_str(v);
                        }
                    }
                    Token::Operation => {
                        if let Some(v) = visitor.fields.get("op") {
                            out.push_str(v);
                        } else if let Some(first) = visitor.msg.split_whitespace().next() {
                            let op = match first.chars().next().unwrap_or(' ') {
                                '>' => "send",
                                '<' => "recv",
                                '*' => "del.",
                                _ => first,
                            };
                            out.push_str(op);
                        }
                    }
                    Token::ModulePath => {
                        if let Some(v) = visitor.fields.get("module_path") {
                            out.push_str(v);
                        }
                    }
                    Token::User => {
                        if let Some(v) = visitor.fields.get("user") {
                            out.push_str(v);
                        }
                    }
                    Token::Uid => {
                        if let Some(v) = visitor.fields.get("uid") {
                            out.push_str(v);
                        }
                    }
                }
            }
            writer.write_str(&out)?;
            writer.write_char('\n')
        } else {
            let msg = if visitor.msg.is_empty() {
                event.metadata().target()
            } else {
                &visitor.msg
            };
            let width = Self::columns();
            let wrapped = Self::wrap(msg, width);
            for (i, line) in wrapped.lines().enumerate() {
                if i > 0 {
                    writer.write_char('\n')?;
                }
                writer.write_str(line)?;
            }
            writer.write_char('\n')
        }
    }
}
