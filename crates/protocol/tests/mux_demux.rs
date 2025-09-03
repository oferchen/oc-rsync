// crates/protocol/tests/mux_demux.rs
use std::thread::sleep;
use std::time::Duration;

use protocol::{demux::Demux, mux::Mux, Message};

#[test]
fn multiplex_multiple_channels() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(100));

    let tx1 = mux.register_channel(1);
    let tx2 = mux.register_channel(2);
    let rx1 = demux.register_channel(1);
    let rx2 = demux.register_channel(2);

    tx1.send(Message::Data(b"one".to_vec())).unwrap();
    tx2.send(Message::Data(b"two".to_vec())).unwrap();

    let mut frames = Vec::new();
    while frames.len() < 2 {
        if let Some(frame) = mux.poll() {
            frames.push(frame);
        }
    }

    for frame in frames {
        demux.ingest(frame).unwrap();
    }

    assert_eq!(rx1.try_recv().unwrap(), Message::Data(b"one".to_vec()));
    assert_eq!(rx2.try_recv().unwrap(), Message::Data(b"two".to_vec()));
}

#[test]
fn keepalive_and_timeout() {
    let keepalive = Duration::from_millis(10);
    let timeout = Duration::from_millis(30);
    let mut mux = Mux::new(keepalive);
    let mut demux = Demux::new(timeout);

    let _tx = mux.register_channel(0);
    let _rx = demux.register_channel(0);

    sleep(Duration::from_millis(20));
    let frame = mux.poll().expect("keepalive frame");
    demux.ingest(frame).unwrap();

    sleep(Duration::from_millis(10));
    assert!(demux.poll().is_ok());

    sleep(Duration::from_millis(40));
    assert!(demux.poll().is_err());
}

#[test]
fn round_robin_fairness() {
    let mut mux = Mux::new(Duration::from_secs(1));

    let tx1 = mux.register_channel(1);
    let tx2 = mux.register_channel(2);

    tx1.send(Message::Data(b"a1".to_vec())).unwrap();
    tx1.send(Message::Data(b"a2".to_vec())).unwrap();
    tx2.send(Message::Data(b"b1".to_vec())).unwrap();
    tx2.send(Message::Data(b"b2".to_vec())).unwrap();

    let mut order = Vec::new();
    for _ in 0..4 {
        let frame = mux.poll().expect("frame");
        order.push(frame.header.channel);
    }

    assert_eq!(order, vec![1, 2, 1, 2]);
}

#[test]
fn unregister_channel_rejects_frames() {
    let mut demux = Demux::new(Duration::from_millis(100));

    let rx = demux.register_channel(1);
    let frame = Message::Data(b"msg".to_vec()).into_frame(1, None);
    demux.ingest(frame).unwrap();
    assert_eq!(rx.try_recv().unwrap(), Message::Data(b"msg".to_vec()));

    demux.unregister_channel(1);
    assert!(rx.try_recv().is_err());

    let frame = Message::Data(b"other".to_vec()).into_frame(1, None);
    assert!(demux.ingest(frame).is_err());
}

#[test]
fn error_xfer_sets_remote_error() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(50));

    mux.register_channel(0);
    let _rx = demux.register_channel(0);

    mux.send_error_xfer(0, "oops").unwrap();
    let frame = mux.poll().expect("frame");
    demux.ingest(frame).unwrap();
    assert_eq!(demux.take_error_xfers(), vec!["oops".to_string()]);
    assert_eq!(demux.take_remote_error(), Some("oops".into()));
}

#[test]
fn progress_attrs_and_xattrs() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(50));

    mux.register_channel(0);
    let rx = demux.register_channel(0);

    mux.send_progress(0, 123).unwrap();
    mux.send_xattrs(0, b"user=1".to_vec()).unwrap();
    mux.send_attrs(0, b"mode=755".to_vec()).unwrap();

    let mut frames = Vec::new();
    while frames.len() < 3 {
        if let Some(frame) = mux.poll() {
            frames.push(frame);
        }
    }

    for frame in frames {
        demux.ingest(frame).unwrap();
    }

    assert_eq!(rx.try_recv().unwrap(), Message::Progress(123));
    assert_eq!(rx.try_recv().unwrap(), Message::Xattrs(b"user=1".to_vec()));
    assert_eq!(
        rx.try_recv().unwrap(),
        Message::Attributes(b"mode=755".to_vec())
    );
}

#[test]
fn collect_log_messages() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(50));

    mux.register_channel(0);
    let _rx = demux.register_channel(0);

    mux.send_info(0, "info").unwrap();
    mux.send_warning(0, "warn").unwrap();
    mux.send_log(0, "log").unwrap();
    mux.send_client(0, "client").unwrap();

    let mut frames = Vec::new();
    while frames.len() < 4 {
        if let Some(frame) = mux.poll() {
            frames.push(frame);
        }
    }

    for frame in frames {
        demux.ingest(frame).unwrap();
    }

    assert_eq!(demux.take_infos(), vec!["info".to_string()]);
    assert_eq!(demux.take_warnings(), vec!["warn".to_string()]);
    assert_eq!(demux.take_logs(), vec!["log".to_string()]);
    assert_eq!(demux.take_clients(), vec!["client".to_string()]);
}

#[test]
fn collect_progress_and_stats_messages() {
    let mut mux = Mux::new(Duration::from_millis(50));
    let mut demux = Demux::new(Duration::from_millis(50));

    mux.register_channel(0);
    let _rx = demux.register_channel(0);

    mux.send_progress(0, 123).unwrap();
    mux.send_stats(0, vec![1, 2, 3]).unwrap();

    let mut frames = Vec::new();
    while frames.len() < 2 {
        if let Some(frame) = mux.poll() {
            frames.push(frame);
        }
    }

    for frame in frames {
        demux.ingest(frame).unwrap();
    }

    assert_eq!(demux.take_progress(), vec![123]);
    assert_eq!(demux.take_stats(), vec![vec![1, 2, 3]]);
}

#[test]
fn poll_with_dynamic_channel_removal() {
    let mut mux = Mux::new(Duration::from_millis(50));

    let tx1 = mux.register_channel(1);
    let tx2 = mux.register_channel(2);

    tx1.send(Message::Data(b"one".to_vec())).unwrap();
    tx2.send(Message::Data(b"two".to_vec())).unwrap();

    let frame = mux.poll().expect("frame");
    assert_eq!(frame.header.channel, 1);

    mux.unregister_channel(2);

    let frame = mux.poll().expect("frame");
    assert_eq!(frame.header.channel, 1);

    mux.unregister_channel(1);
    assert!(mux.poll().is_none());
}
