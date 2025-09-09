// crates/daemon/tests/sequential_connections.rs
use daemon::{Handler, Module, handle_connection};
#[cfg(unix)]
use nix::unistd::geteuid;
use protocol::SUPPORTED_PROTOCOLS;
use std::collections::HashMap;
use std::io::{self, Cursor, Read};
use std::sync::Arc;
use tempfile::tempdir;
use transport::LocalPipeTransport;

#[test]
fn handle_sequential_chrooted_connections() {
    #[cfg(unix)]
    if geteuid().as_raw() != 0 {
        eprintln!("skipping handle_sequential_chrooted_connections: requires root");
        return;
    }
    let dir = tempdir().unwrap();
    let module = Module {
        name: "data".to_string(),
        path: dir.path().to_path_buf(),
        uid: Some(1),
        gid: Some(1),
        use_chroot: true,
        ..Default::default()
    };
    let mut modules = HashMap::new();
    modules.insert(module.name.clone(), module);
    let handler: Arc<Handler> = Arc::new(|_, _| Ok(()));
    let cwd = std::env::current_dir().unwrap();
    struct MultiReader {
        parts: Vec<Vec<u8>>,
        idx: usize,
        pos: usize,
    }

    impl Read for MultiReader {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            if self.idx >= self.parts.len() {
                return Ok(0);
            }
            let part = &self.parts[self.idx];
            let remaining = &part[self.pos..];
            let len = remaining.len().min(buf.len());
            buf[..len].copy_from_slice(&remaining[..len]);
            self.pos += len;
            if self.pos >= part.len() {
                self.idx += 1;
                self.pos = 0;
            }
            Ok(len)
        }
    }

    for _ in 0..3 {
        let parts = vec![
            SUPPORTED_PROTOCOLS[0].to_be_bytes().to_vec(),
            b"auth\n".to_vec(),
            b"data\n\n".to_vec(),
        ];
        let reader = MultiReader {
            parts,
            idx: 0,
            pos: 0,
        };
        let writer = Cursor::new(Vec::new());
        let mut transport = LocalPipeTransport::new(reader, writer);
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
        .expect("connection should succeed");
        let (_, writer) = transport.into_inner();
        let out = writer.into_inner();
        assert_eq!(&out[4..], b"@RSYNCD: OK\n@RSYNCD: OK\n@RSYNCD: EXIT\n",);
        assert_eq!(std::env::current_dir().unwrap(), cwd);
    }
}
