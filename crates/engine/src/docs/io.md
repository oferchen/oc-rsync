# I/O Preallocation Safety

The `preallocate` function performs platform-specific file preallocation.
It contains unsafe blocks that invoke raw `libc` functions.  The following
preconditions must hold for those calls to be sound.

## macOS (`fcntl`/`ftruncate`)

- `file` is a valid, open file and its descriptor remains valid for the
  duration of the call.
- `len` does not exceed the range of `libc::off_t`.
- The `libc::fstore_t` structure passed to `fcntl` is fully initialized.
- Return values from `fcntl` and `ftruncate` are checked and any errors are
  propagated as `io::Error`.

## *BSD, illumos, and Solaris (`posix_fallocate`)

- `file` provides a valid file descriptor for the lifetime of the call.
- `len` fits within `libc::off_t` and is non-negative.
- The return code from `posix_fallocate` is examined and converted into
  an `io::Error` on failure.

Violating these preconditions would result in undefined behavior.
