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

    // Collect two frames from the multiplexer
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

    // Register channel 0
    let _tx = mux.register_channel(0);
    let _rx = demux.register_channel(0);

    // No data is sent, after the keepalive interval the mux should emit a keepalive
    sleep(Duration::from_millis(20));
    let frame = mux.poll().expect("keepalive frame");
    demux.ingest(frame).unwrap();

    // Keepalive resets timer; no timeout yet
    sleep(Duration::from_millis(10));
    assert!(demux.poll().is_ok());

    // Without further frames we should eventually time out
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
