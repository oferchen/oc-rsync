// crates/protocol/tests/demux_limits.rs
use protocol::{Demux, Message};
use std::time::Duration;

#[test]
fn limits_info_capacity() {
    let mut demux = Demux::with_capacity(Duration::from_secs(1), 5);
    let _rx = demux.register_channel(0);
    for i in 0..100 {
        demux
            .ingest_message(0, Message::Info(i.to_string()))
            .unwrap();
    }
    let infos = demux.take_infos();
    assert_eq!(infos.len(), 5);
    let expected: Vec<String> = (95..100).map(|i| i.to_string()).collect();
    assert_eq!(infos, expected);
}
