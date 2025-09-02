// crates/protocol/tests/messages.rs
use protocol::Message;

#[test]
fn roundtrip_additional_messages() {
    let msgs = [
        Message::ErrorXfer("test".into()),
        Message::Info("test".into()),
        Message::Warning("test".into()),
        Message::ErrorSocket("test".into()),
        Message::ErrorUtf8("test".into()),
        Message::Log("test".into()),
        Message::Client("test".into()),
        Message::IoError(7),
        Message::IoTimeout(9),
        Message::Noop,
    ];
    for msg in msgs.into_iter() {
        let frame = msg.clone().to_frame(9, None);
        let decoded = Message::from_frame(frame, None).unwrap();
        assert_eq!(decoded, msg);
    }
}
