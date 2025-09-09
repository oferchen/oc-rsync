// crates/transport/src/config.rs
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfig {
    pub timeout: Option<Duration>,

    pub retries: u32,

    pub rate_limit: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportConfigError(&'static str);

impl std::fmt::Display for TransportConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TransportConfigError {}

pub type Result<T> = std::result::Result<T, TransportConfigError>;

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
    pub fn builder() -> TransportConfigBuilder {
        TransportConfigBuilder::default()
    }
}

impl TransportConfigBuilder {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    pub fn rate_limit(mut self, limit: u64) -> Self {
        self.rate_limit = Some(limit);
        self
    }

    pub fn build(self) -> Result<TransportConfig> {
        if self.timeout.is_some_and(|t| t.is_zero()) {
            return Err(TransportConfigError("timeout must be nonzero"));
        }
        if self.rate_limit.is_some_and(|rl| rl == 0) {
            return Err(TransportConfigError("rate limit must be nonzero"));
        }
        Ok(TransportConfig {
            timeout: self.timeout,
            retries: self.retries,
            rate_limit: self.rate_limit,
        })
    }
}
