// crates/protocol/tests/mux_register.rs
use std::time::Duration;

use protocol::{mux::ChannelError, Message, Mux};

#[test]
fn duplicate_channel_id_errors() {
    let mut mux = Mux::new(Duration::from_millis(50));

    let tx1 = mux
        .register_channel(1)
        .expect("first registration succeeds");
    assert_eq!(
        mux.register_channel(1),
        Err(ChannelError::DuplicateId(1)),
        "second registration should fail",
    );

    tx1.send(Message::Data(b"hi".to_vec())).unwrap();

    let frame = mux.poll().expect("frame from existing channel");
    assert_eq!(frame.header.channel, 1);
    let msg = Message::from_frame(frame, None).unwrap();
    assert_eq!(msg, Message::Data(b"hi".to_vec()));
}
