// crates/daemon/tests/no_token.rs
use daemon::{Handler, Module, handle_connection};
use protocol::SUPPORTED_PROTOCOLS;
use std::collections::HashMap;
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::Arc;
use transport::LocalPipeTransport;

#[test]
fn handle_connection_empty_module_name_lists_modules() {
    let mut input = Vec::new();
    input.extend_from_slice(&SUPPORTED_PROTOCOLS[0].to_be_bytes());
    input.extend_from_slice(b"auth\n\n\n");
    let reader = Cursor::new(input);
    let writer = Cursor::new(Vec::new());
    let mut transport = LocalPipeTransport::new(reader, writer);

    let mut modules: HashMap<String, Module> = HashMap::new();
    modules.insert(
        "foo".to_string(),
        Module {
            name: "foo".to_string(),
            path: PathBuf::from("."),
            list: true,
            ..Module::default()
        },
    );
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));

    handle_connection(
        &mut transport,
        &modules,
        None,
        None,
        None,
        None,
        None,
        true,
        &[],
        "127.0.0.1",
        0,
        0,
        &handler,
        None,
    )
    .expect("empty module name should list modules");

    let (_, writer) = transport.into_inner();
    let out = writer.into_inner();
    assert_eq!(&out[4..], b"@RSYNCD: OK\nfoo\n\n");
}
