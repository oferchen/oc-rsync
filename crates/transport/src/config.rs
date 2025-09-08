use std::time::Duration;

/// Configuration for transport behavior such as timeouts, retries and rate
/// limiting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfig {
    /// Timeout for I/O operations. `None` disables the timeout.
    pub timeout: Option<Duration>,
    /// Number of times to retry operations on failure.
    pub retries: u32,
    /// Optional rate limit in bytes per second.
    pub rate_limit: Option<u64>,
}

/// Error produced when building an invalid [`TransportConfig`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfigError(&'static str);

impl std::fmt::Display for TransportConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TransportConfigError {}

/// Result type used by the [`TransportConfig`] builder.
pub type Result<T> = std::result::Result<T, TransportConfigError>;

/// Builder for [`TransportConfig`].
#[derive(Debug, Clone)]
pub struct TransportConfigBuilder {
    timeout: Option<Duration>,
    retries: u32,
    rate_limit: Option<u64>,
}

impl Default for TransportConfigBuilder {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(30)),
            retries: 3,
            rate_limit: None,
        }
    }
}

impl TransportConfig {
    /// Create a new builder pre-populated with default values.
    pub fn builder() -> TransportConfigBuilder {
        TransportConfigBuilder::default()
    }
}

impl TransportConfigBuilder {
    /// Set an I/O timeout. A zero duration is rejected.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Disable I/O timeouts.
    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    /// Set the retry count.
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set a rate limit in bytes per second. Zero is rejected.
    pub fn rate_limit(mut self, limit: u64) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    /// Build the [`TransportConfig`], validating invariants.
    pub fn build(self) -> Result<TransportConfig> {
        if let Some(t) = self.timeout
            && t.is_zero()
        {
            return Err(TransportConfigError("timeout must be nonzero"));
        }
        if let Some(rl) = self.rate_limit
            && rl == 0
        {
            return Err(TransportConfigError("rate limit must be nonzero"));
        }
        Ok(TransportConfig {
            timeout: self.timeout,
            retries: self.retries,
            rate_limit: self.rate_limit,
        })
    }
}
