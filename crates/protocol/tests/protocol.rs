// crates/protocol/tests/protocol.rs
use encoding_rs::Encoding;
use filelist::{Decoder as FDecoder, Encoder as FEncoder, Entry as FEntry};
use protocol::{
    CAP_ACLS, CAP_CODECS, CAP_XATTRS, CAP_ZSTD, CharsetConv, Frame, MIN_VERSION, Message, Msg,
    SUPPORTED_PROTOCOLS, Tag, negotiate_caps, negotiate_version,
};
use std::io;

#[test]
fn frame_roundtrip() -> io::Result<()> {
    let msg = Message::Data(b"hello".to_vec());
    let frame = msg.to_frame(5, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded, frame);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg4 = Message::Data(b"1234".to_vec());
    let frame4 = msg4.to_frame(3, None);
    let mut buf4 = Vec::new();
    frame4.encode(&mut buf4).unwrap();
    let mut cursor = &buf4[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded, frame4);
    let msg4_round = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg4_round, msg4);
    Ok(())
}

#[test]
fn keepalive_roundtrip() -> io::Result<()> {
    let msg = Message::KeepAlive;
    let frame = msg.to_frame(0, None);
    assert_eq!(frame.header.tag, Tag::KeepAlive);
    assert_eq!(frame.header.msg, Msg::KeepAlive);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded, frame);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, Message::KeepAlive);
    Ok(())
}

#[test]
fn version_negotiation() {
    let latest = SUPPORTED_PROTOCOLS[0];
    for &peer in SUPPORTED_PROTOCOLS {
        assert_eq!(negotiate_version(latest, peer), Ok(peer));
        assert_eq!(negotiate_version(peer, latest), Ok(peer));
    }
    assert!(negotiate_version(latest, MIN_VERSION - 1).is_err());
}

#[test]
fn capability_negotiation() {
    let local = CAP_CODECS | CAP_ACLS | CAP_XATTRS;
    let peer = CAP_CODECS | CAP_ZSTD | CAP_XATTRS;
    assert_eq!(negotiate_caps(local, peer), CAP_CODECS | CAP_XATTRS);
}

#[test]
fn captured_frames_roundtrip() -> io::Result<()> {
    let entry = FEntry {
        path: b"file.txt".to_vec(),
        uid: 0,
        gid: 0,
        hardlink: None,
        xattrs: vec![(b"user.test".to_vec(), b"1".to_vec())],
        acl: vec![1, 0, 0, 0, 0, 7, 0, 0, 0],
        default_acl: Vec::new(),
    };
    let mut fenc = FEncoder::new();
    let payload = fenc.encode_entry(&entry);
    let mut expected = Vec::new();
    Frame {
        header: protocol::FrameHeader {
            channel: 0,
            tag: Tag::Message,
            msg: Msg::FileListEntry,
            len: payload.len() as u32,
        },
        payload: payload.clone(),
    }
    .encode(&mut expected)
    .unwrap();
    let mut cursor = &expected[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::FileListEntry);
    let msg = Message::from_frame(decoded.clone(), None).unwrap();
    assert_eq!(msg, Message::FileListEntry(payload.clone()));
    let mut buf = Vec::new();
    Message::FileListEntry(payload.clone())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, expected);
    let mut fdec = FDecoder::new();
    assert_eq!(msg.to_file_list(&mut fdec, None).unwrap(), entry);

    const ATTRS: [u8; 16] = [
        0, 0, 0, 244, 0, 0, 0, 8, b'm', b'o', b'd', b'e', b'=', b'7', b'5', b'5',
    ];
    let mut cursor = &ATTRS[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Attributes);
    let msg = Message::from_frame(decoded.clone(), None).unwrap();
    assert_eq!(msg, Message::Attributes(b"mode=755".to_vec()));
    let mut buf = Vec::new();
    Message::Attributes(b"mode=755".to_vec())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, ATTRS);

    const ERR: [u8; 12] = [0, 0, 0, 3, 0, 0, 0, 4, b'o', b'o', b'p', b's'];
    let mut cursor = &ERR[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Error);
    let msg = Message::from_frame(decoded.clone(), None).unwrap();
    assert_eq!(msg, Message::Error("oops".to_string()));
    let mut buf = Vec::new();
    Message::Error("oops".to_string())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, ERR);

    const PROG: [u8; 16] = [0, 0, 0, 245, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0x30, 0x39];
    let mut cursor = &PROG[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Progress);
    let msg = Message::from_frame(decoded.clone(), None).unwrap();
    assert_eq!(msg, Message::Progress(0x3039));
    let mut buf = Vec::new();
    Message::Progress(0x3039)
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, PROG);

    const XATTRS: [u8; 14] = [
        0, 0, 0, 0xF7, 0, 0, 0, 6, b'u', b's', b'e', b'r', b'=', b'1',
    ];
    let mut cursor = &XATTRS[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Xattrs);
    let msg = Message::from_frame(decoded.clone(), None).unwrap();
    assert_eq!(msg, Message::Xattrs(b"user=1".to_vec()));
    let mut buf = Vec::new();
    Message::Xattrs(b"user=1".to_vec())
        .into_frame(0, None)
        .encode(&mut buf)
        .unwrap();
    assert_eq!(buf, XATTRS);
    Ok(())
}

#[test]
fn extra_messages_roundtrip() -> io::Result<()> {
    let msg = Message::Codecs(vec![1, 2, 3]);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Codecs);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Redo(0x01020304);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Redo);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Stats(vec![1, 2, 3]);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Stats);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Success(7);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Success);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Deleted(9);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Deleted);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::NoSend(11);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::NoSend);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Exit(2);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::ErrorExit);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);
    Ok(())
}

#[test]
fn demux_error_routing() {
    use protocol::{Demux, Mux};
    use std::time::Duration;

    let mut mux = Mux::new(Duration::from_secs(1));
    let tx = mux.register_channel(0).unwrap();
    tx.send(Message::ErrorXfer("ex".into())).unwrap();
    tx.send(Message::Error("er".into())).unwrap();
    tx.send(Message::ErrorSocket("so".into())).unwrap();
    tx.send(Message::ErrorUtf8("ut".into())).unwrap();

    let mut demux = Demux::new(Duration::from_secs(1));
    let _rx = demux.register_channel(0);
    for _ in 0..4 {
        let frame = mux.poll().unwrap();
        demux.ingest(frame).unwrap();
    }

    assert_eq!(demux.take_error_xfers(), vec!["ex".to_string()]);
    assert_eq!(demux.take_errors(), vec!["er".to_string()]);
    assert_eq!(demux.take_error_sockets(), vec!["so".to_string()]);
    assert_eq!(demux.take_error_utf8s(), vec!["ut".to_string()]);
}

#[test]
fn log_messages_roundtrip() -> io::Result<()> {
    let texts = [
        (Message::ErrorXfer("a".into()), Msg::ErrorXfer),
        (Message::Info("b".into()), Msg::Info),
        (Message::Warning("c".into()), Msg::Warning),
        (Message::ErrorSocket("d".into()), Msg::ErrorSocket),
        (Message::Log("e".into()), Msg::Log),
        (Message::Client("f".into()), Msg::Client),
        (Message::ErrorUtf8("g".into()), Msg::ErrorUtf8),
    ];
    for (msg, code) in texts.into_iter() {
        let frame = msg.to_frame(0, None);
        let mut buf = Vec::new();
        frame.encode(&mut buf).unwrap();
        let mut cursor = &buf[..];
        let decoded = Frame::decode(&mut cursor)?;
        assert_eq!(decoded.header.msg, code);
        let round = Message::from_frame(decoded, None).unwrap();
        assert_eq!(round, msg);
    }

    let msg = Message::IoError(123);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::IoError);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::IoTimeout(456);
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::IoTimeout);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);

    let msg = Message::Noop;
    let frame = msg.to_frame(0, None);
    let mut buf = Vec::new();
    frame.encode(&mut buf).unwrap();
    let mut cursor = &buf[..];
    let decoded = Frame::decode(&mut cursor)?;
    assert_eq!(decoded.header.msg, Msg::Noop);
    let msg2 = Message::from_frame(decoded, None).unwrap();
    assert_eq!(msg2, msg);
    Ok(())
}

#[test]
fn error_message_iconv_roundtrip() {
    let cv = CharsetConv::new(
        Encoding::for_label(b"latin1").unwrap(),
        Encoding::for_label(b"utf-8").unwrap(),
    );
    let msg = Message::Error("Grüße".into());
    let frame = msg.to_frame(0, Some(&cv));
    let decoded = Message::from_frame(frame, Some(&cv)).unwrap();
    assert_eq!(decoded, msg);
}

#[test]
fn filelist_iconv_roundtrip() {
    let cv = CharsetConv::new(
        Encoding::for_label(b"latin1").unwrap(),
        Encoding::for_label(b"utf-8").unwrap(),
    );
    let entry = FEntry {
        path: "Grüße".as_bytes().to_vec(),
        uid: 0,
        gid: 0,
        hardlink: None,
        xattrs: Vec::new(),
        acl: Vec::new(),
        default_acl: Vec::new(),
    };
    let mut enc = FEncoder::new();
    let msg = Message::from_file_list(&entry, &mut enc, Some(&cv));
    let mut dec = FDecoder::new();
    let round = msg.to_file_list(&mut dec, Some(&cv)).unwrap();
    assert_eq!(round, entry);
}
