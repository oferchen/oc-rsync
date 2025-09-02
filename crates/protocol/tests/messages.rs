// crates/protocol/tests/messages.rs
use protocol::{Message, Msg};

#[test]
fn roundtrip_additional_messages() {
    let msgs = [
        Msg::ErrorXfer,
        Msg::Info,
        Msg::Warning,
        Msg::ErrorSocket,
        Msg::ErrorUtf8,
        Msg::Log,
        Msg::Client,
        Msg::Redo,
        Msg::Stats,
        Msg::IoError,
        Msg::IoTimeout,
        Msg::Noop,
        Msg::ErrorExit,
        Msg::Success,
        Msg::Deleted,
        Msg::NoSend,
    ];
    for m in msgs {
        let payload = b"test".to_vec();
        let msg = Message::Other(m, payload.clone());
        let frame = msg.clone().to_frame(9, None);
        let decoded = Message::from_frame(frame, None).unwrap();
        assert_eq!(decoded, msg);
    }
}
