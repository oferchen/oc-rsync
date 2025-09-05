// crates/daemon/tests/no_token.rs
use daemon::{handle_connection, Handler, Module};
use protocol::SUPPORTED_PROTOCOLS;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use transport::LocalPipeTransport;

#[test]
fn handle_connection_empty_module_name() {
    let mut input = Vec::new();
    input.extend_from_slice(&SUPPORTED_PROTOCOLS[0].to_be_bytes());
    input.extend_from_slice(b"\n\n");
    let reader = Cursor::new(input);
    let writer = Cursor::new(Vec::new());
    let mut transport = LocalPipeTransport::new(reader, writer);

    let modules: HashMap<String, Module> = HashMap::new();
    let handler: Arc<Handler> = Arc::new(|_t| Ok(()));

    handle_connection(
        &mut transport,
        &modules,
        None,
        None,
        None,
        None,
        None,
        false,
        &[],
        "127.0.0.1",
        0,
        0,
        &handler,
        None,
    )
    .expect("empty module name should succeed");

    let (_, writer) = transport.into_inner();
    let out = writer.into_inner();
    assert_eq!(&out[4..], b"@RSYNCD: OK\n\n");
}
