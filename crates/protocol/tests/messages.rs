// crates/protocol/tests/messages.rs
use protocol::Message;

#[test]
fn roundtrip_additional_messages() {
    let msgs = [
        Message::ErrorXfer("test".into()),
        Message::Error("test".into()),
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

#[test]
fn roundtrip_remaining_messages() {
    let msgs = [
        Message::Success(1),
        Message::Deleted(2),
        Message::NoSend(3),
        Message::Redo(4),
        Message::Stats(vec![1, 2, 3]),
        Message::Progress(5),
        Message::Attributes(vec![6, 7]),
        Message::FileListEntry(vec![8, 9]),
        Message::Codecs(vec![10, 11]),
        Message::Exit(12),
        Message::Done,
        Message::KeepAlive,
    ];
    for msg in msgs.into_iter() {
        let frame = msg.clone().to_frame(1, None);
        let decoded = Message::from_frame(frame, None).unwrap();
        assert_eq!(decoded, msg);
    }
}
