// crates/logging/src/formatter.rs
use std::fmt;
use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber};
use tracing_subscriber::fmt::{format::Writer, FmtContext, FormatEvent, FormatFields};
use tracing_subscriber::registry::LookupSpan;

pub struct RsyncFormatter;

impl RsyncFormatter {
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

struct MsgVisitor {
    msg: String,
}

impl MsgVisitor {
    fn new() -> Self {
        Self { msg: String::new() }
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
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(field.name());
            self.msg.push('=');
            self.msg.push_str(value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn fmt::Debug) {
        if field.name() == "message" {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(&format!("{value:?}"));
        } else {
            if !self.msg.is_empty() {
                self.msg.push(' ');
            }
            self.msg.push_str(field.name());
            self.msg.push('=');
            self.msg.push_str(&format!("{value:?}"));
        }
    }
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
