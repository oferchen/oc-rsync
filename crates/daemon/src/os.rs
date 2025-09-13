// crates/daemon/src/os.rs
#![allow(unsafe_code)]

#[cfg(unix)]
use nix::unistd::{ForkResult, fork};

#[cfg(unix)]
/// Fork the current process.
///
/// # Safety
/// This wrapper is safe because it performs the raw `fork(2)` and returns
/// immediately without touching shared state in the child. The caller is
/// responsible for performing only async-signal-safe operations before any
/// further library calls.
pub(crate) fn fork_daemon() -> nix::Result<ForkResult> {
    // SAFETY: see the `Safety` section above. We do not access shared state in
    // the child before returning to the caller.
    unsafe { fork() }
}
