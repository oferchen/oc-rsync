// Reuse the core remote-remote tests in the interop test suite so CI exercises
// them against real transports as well.
include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/remote_remote.rs"));
