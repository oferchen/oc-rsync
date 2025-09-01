// crates/protocol/tests/exit_codes.rs
use protocol::{Demux, ExitCode, Message, Mux};
use std::convert::TryFrom;
use std::time::Duration;

#[test]
fn exit_code_roundtrip() {
    let codes = [
        (0u8, ExitCode::Ok),
        (1, ExitCode::SyntaxOrUsage),
        (2, ExitCode::Protocol),
        (23, ExitCode::Partial),
        (127, ExitCode::CmdNotFound),
    ];
    for (num, code) in codes {
        assert_eq!(ExitCode::try_from(num).unwrap(), code);
        let back: u8 = code.into();
        assert_eq!(back, num);
    }
}

#[test]
fn unknown_exit_code_errors() {
    assert!(ExitCode::try_from(99u8).is_err());
}

#[test]
fn forward_exit_codes_over_mux_demux() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(50));

    let tx = mux.register_channel(1);
    let rx = demux.register_channel(1);

    let codes = [ExitCode::Ok, ExitCode::Partial, ExitCode::CmdNotFound];

    for code in codes {
        let byte: u8 = code.into();
        tx.send(Message::Data(vec![byte])).unwrap();

        let frame = mux.poll().expect("frame");
        demux.ingest(frame).unwrap();
        match rx.try_recv().expect("message") {
            Message::Data(payload) => {
                assert_eq!(payload, vec![byte]);
                let received = ExitCode::try_from(payload[0]).unwrap();
                assert_eq!(received, code);
            }
            other => panic!("unexpected message: {:?}", other),
        }
    }
}

#[test]
fn forward_unknown_exit_code_over_mux_demux() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(50));

    let tx = mux.register_channel(1);
    let rx = demux.register_channel(1);

    let byte = 99u8;
    tx.send(Message::Data(vec![byte])).unwrap();

    let frame = mux.poll().expect("frame");
    demux.ingest(frame).unwrap();
    match rx.try_recv().expect("message") {
        Message::Data(payload) => {
            assert_eq!(payload, vec![byte]);
            assert!(ExitCode::try_from(payload[0]).is_err());
        }
        other => panic!("unexpected message: {:?}", other),
    }
}

#[test]
fn demux_nonzero_exit_errors() {
    let mut demux = Demux::new(Duration::from_millis(50));
    let frame = Message::Data(vec![1]).to_frame(0);
    let err = demux.ingest(frame).unwrap_err();
    assert_eq!(demux.take_exit_code(), Some(1));
    assert_eq!(err.kind(), std::io::ErrorKind::Other);
}

#[test]
fn demux_remote_error_propagates() {
    let mut demux = Demux::new(Duration::from_millis(50));
    let frame = Message::Error("oops".into()).to_frame(5);
    let err = demux.ingest(frame).unwrap_err();
    assert_eq!(err.kind(), std::io::ErrorKind::Other);
    assert_eq!(demux.take_remote_error().as_deref(), Some("oops"));
}
