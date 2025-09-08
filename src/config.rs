// src/config.rs

use logging::{DebugFlag, InfoFlag, LogFormat};
use std::path::PathBuf;

/// Configuration for a synchronization run.
#[derive(Clone, Debug)]
pub struct SyncConfig {
    pub log_format: LogFormat,
    pub verbose: u8,
    pub info: Vec<InfoFlag>,
    pub debug: Vec<DebugFlag>,
    pub quiet: bool,
    pub log_file: Option<(PathBuf, Option<String>)>,
    pub syslog: bool,
    pub journald: bool,
    pub colored: bool,
    pub timestamps: bool,
    pub perms: bool,
    pub times: bool,
    pub atimes: bool,
    pub open_noatime: bool,
    pub links: bool,
    pub devices: bool,
    pub specials: bool,
    pub owner: bool,
    pub group: bool,
    #[cfg(feature = "xattr")]
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    pub acls: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            log_format: LogFormat::Text,
            verbose: 0,
            info: Vec::new(),
            debug: Vec::new(),
            quiet: false,
            log_file: None,
            syslog: false,
            journald: false,
            colored: true,
            timestamps: false,
            perms: false,
            times: false,
            atimes: false,
            open_noatime: false,
            links: false,
            devices: false,
            specials: false,
            owner: false,
            group: false,
            #[cfg(feature = "xattr")]
            xattrs: false,
            #[cfg(feature = "acl")]
            acls: false,
        }
    }
}

impl SyncConfig {
    /// Create a new builder for [`SyncConfig`].
    pub fn builder() -> SyncConfigBuilder {
        SyncConfigBuilder::default()
    }
}

/// Builder for [`SyncConfig`].
#[derive(Debug, Default)]
#[must_use]
pub struct SyncConfigBuilder {
    cfg: SyncConfig,
}

impl SyncConfigBuilder {
    pub fn log_format(mut self, log_format: LogFormat) -> Self {
        self.cfg.log_format = log_format;
        self
    }

    pub fn verbose(mut self, verbose: u8) -> Self {
        self.cfg.verbose = verbose;
        self
    }

    pub fn info<I>(mut self, info: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<InfoFlag>,
    {
        let mut info = info.into_iter().map(Into::into).collect::<Vec<_>>();
        info.sort_by_key(|flag| flag.as_str());
        info.dedup();
        self.cfg.info = info;
        self
    }

    pub fn debug<I>(mut self, debug: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<DebugFlag>,
    {
        let mut debug = debug.into_iter().map(Into::into).collect::<Vec<_>>();
        debug.sort_by_key(|flag| flag.as_str());
        debug.dedup();
        self.cfg.debug = debug;
        self
    }

    pub fn quiet(mut self, quiet: bool) -> Self {
        self.cfg.quiet = quiet;
        self
    }

    pub fn log_file(mut self, log_file: Option<(PathBuf, Option<String>)>) -> Self {
        self.cfg.log_file = log_file;
        self
    }

    pub fn syslog(mut self, enable: bool) -> Self {
        self.cfg.syslog = enable;
        self
    }

    pub fn journald(mut self, enable: bool) -> Self {
        self.cfg.journald = enable;
        self
    }

    pub fn colored(mut self, enable: bool) -> Self {
        self.cfg.colored = enable;
        self
    }

    pub fn timestamps(mut self, enable: bool) -> Self {
        self.cfg.timestamps = enable;
        self
    }

    pub fn perms(mut self, enable: bool) -> Self {
        self.cfg.perms = enable;
        self
    }

    pub fn times(mut self, enable: bool) -> Self {
        self.cfg.times = enable;
        self
    }

    pub fn atimes(mut self, enable: bool) -> Self {
        self.cfg.atimes = enable;
        self
    }

    pub fn open_noatime(mut self, enable: bool) -> Self {
        self.cfg.open_noatime = enable;
        self
    }

    pub fn links(mut self, enable: bool) -> Self {
        self.cfg.links = enable;
        self
    }

    pub fn devices(mut self, enable: bool) -> Self {
        self.cfg.devices = enable;
        self
    }

    pub fn specials(mut self, enable: bool) -> Self {
        self.cfg.specials = enable;
        self
    }

    pub fn owner(mut self, enable: bool) -> Self {
        self.cfg.owner = enable;
        self
    }

    pub fn group(mut self, enable: bool) -> Self {
        self.cfg.group = enable;
        self
    }

    #[cfg(feature = "xattr")]
    pub fn xattrs(mut self, enable: bool) -> Self {
        self.cfg.xattrs = enable;
        self
    }

    #[cfg(feature = "acl")]
    pub fn acls(mut self, enable: bool) -> Self {
        self.cfg.acls = enable;
        self
    }

    pub fn build(self) -> SyncConfig {
        self.cfg
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn info_dedupes_flags() {
        let cfg = SyncConfig::builder()
            .info([InfoFlag::Backup, InfoFlag::Backup])
            .build();
        assert_eq!(cfg.info.len(), 1);
        assert!(cfg.info.contains(&InfoFlag::Backup));
    }

    #[test]
    fn debug_dedupes_flags() {
        let cfg = SyncConfig::builder()
            .debug([DebugFlag::Backup, DebugFlag::Backup])
            .build();
        assert_eq!(cfg.debug.len(), 1);
        assert!(cfg.debug.contains(&DebugFlag::Backup));
    }

    #[test]
    fn info_flag_order_deterministic() {
        let flags = [InfoFlag::Del, InfoFlag::Backup, InfoFlag::Copy];
        let cfg1 = SyncConfig::builder().info(flags).build();
        let cfg2 = SyncConfig::builder().info(flags).build();
        assert_eq!(cfg1.info, cfg2.info);
    }

    #[test]
    fn debug_flag_order_deterministic() {
        let flags = [DebugFlag::Recv, DebugFlag::Backup, DebugFlag::Send];
        let cfg1 = SyncConfig::builder().debug(flags).build();
        let cfg2 = SyncConfig::builder().debug(flags).build();
        assert_eq!(cfg1.debug, cfg2.debug);
    }

    #[test]
    fn info_repeated_calls_overwrite() {
        let cfg = SyncConfig::builder()
            .info([InfoFlag::Backup])
            .info([InfoFlag::Backup, InfoFlag::Backup])
            .build();
        assert_eq!(cfg.info.len(), 1);
        assert!(cfg.info.contains(&InfoFlag::Backup));
    }

    #[test]
    fn debug_repeated_calls_overwrite() {
        let cfg = SyncConfig::builder()
            .debug([DebugFlag::Backup])
            .debug([DebugFlag::Backup, DebugFlag::Backup])
            .build();
        assert_eq!(cfg.debug.len(), 1);
        assert!(cfg.debug.contains(&DebugFlag::Backup));
    }
}
