//! Shared helper utilities for fuzzing targets.

/// Utilities that are reused across fuzz targets.
///
/// Keeping the helpers in a separate module makes it easy for each
/// fuzz target to pull in the small bits of functionality it needs
/// without repeating boilerplate.
pub mod helpers {
    use std::io::Cursor;

    /// Wrap the provided byte slice in a [`Cursor`].
    ///
    /// Many fuzz targets operate on types that expect an `io::Read`
    /// implementation.  A `Cursor` over the input bytes satisfies that
    /// requirement without allocating.
    #[inline]
    pub fn cursor(data: &[u8]) -> Cursor<&[u8]> {
        Cursor::new(data)
    }

    /// Attempt to interpret the provided byte slice as UTF‑8.
    ///
    /// Returning `None` instead of panicking keeps fuzz targets simple
    /// when they need optional textual input.
    #[inline]
    pub fn as_str(data: &[u8]) -> Option<&str> {
        std::str::from_utf8(data).ok()
    }
}
