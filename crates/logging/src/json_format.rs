// crates/logging/src/json_format.rs
#![allow(missing_docs)]

use serde_json::{Map, Value};
use time::OffsetDateTime;
use tracing::{Event, Subscriber};
use tracing_serde::{AsSerde, fields::AsMap};
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, format::Writer};
use tracing_subscriber::registry::LookupSpan;
#[derive(Default)]
pub struct JsonFormatter;

impl<S, N> FormatEvent<S, N> for JsonFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        _ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &Event<'_>,
    ) -> std::fmt::Result {
        let mut obj = Map::new();
        let timestamp = OffsetDateTime::now_utc()
            .format(&time::macros::format_description!(
                "[year]-[month]-[day]T[hour]:[minute]:[second]Z"
            ))
            .map_err(|_| std::fmt::Error)?;
        obj.insert("timestamp".into(), Value::String(timestamp));
        obj.insert(
            "level".into(),
            serde_json::to_value(event.metadata().level().as_serde())
                .map_err(|_| std::fmt::Error)?,
        );
        obj.insert(
            "target".into(),
            Value::String(event.metadata().target().to_string()),
        );
        let fields = serde_json::to_value(event.field_map()).map_err(|_| std::fmt::Error)?;
        obj.insert("fields".into(), fields);
        let json = Value::Object(obj);
        let data = serde_json::to_string(&json).map_err(|_| std::fmt::Error)?;
        writer.write_str(&data)?;
        writer.write_char('\n')
    }
}
