// src/lib.rs

use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use logging::{subscriber, DebugFlag, InfoFlag, LogFormat, SubscriberConfig};
use std::path::{Path, PathBuf};
use tracing::subscriber::with_default;

/// Configuration controlling how `oc-rsync` synchronizes files and emits log
/// output.
#[derive(Clone)]
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
    pub links: bool,
    pub devices: bool,
    pub specials: bool,
    pub owner: bool,
    pub group: bool,
    #[cfg(feature = "xattr")]
    /// Preserve extended attributes. Requires the `xattr` feature.
    pub xattrs: bool,
    #[cfg(feature = "acl")]
    /// Preserve POSIX ACLs. Requires the `acl` feature.
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
    /// Returns a [`SyncConfigBuilder`] to construct a [`SyncConfig`].
    pub fn builder() -> SyncConfigBuilder {
        SyncConfigBuilder::default()
    }
}

/// Builder for [`SyncConfig`].
#[derive(Default)]
#[must_use]
pub struct SyncConfigBuilder {
    cfg: SyncConfig,
}

impl SyncConfigBuilder {
    /// Sets the desired output format for log records.
    pub fn log_format(mut self, log_format: LogFormat) -> Self {
        self.cfg.log_format = log_format;
        self
    }

    /// Sets the verbosity level for logging.
    pub fn verbose(mut self, verbose: u8) -> Self {
        self.cfg.verbose = verbose;
        self
    }

    /// Replaces the set of informational logging flags.
    pub fn info(mut self, info: Vec<InfoFlag>) -> Self {
        self.cfg.info = info;
        self
    }

    /// Replaces the set of debugging logging flags.
    pub fn debug(mut self, debug: Vec<DebugFlag>) -> Self {
        self.cfg.debug = debug;
        self
    }

    /// Silences all non-error log output when `true`.
    pub fn quiet(mut self, quiet: bool) -> Self {
        self.cfg.quiet = quiet;
        self
    }

    /// Logs to the specified file instead of standard error.
    pub fn log_file(mut self, log_file: Option<(PathBuf, Option<String>)>) -> Self {
        self.cfg.log_file = log_file;
        self
    }

    /// Enables syslog output.
    pub fn syslog(mut self, enable: bool) -> Self {
        self.cfg.syslog = enable;
        self
    }

    /// Enables journald output.
    pub fn journald(mut self, enable: bool) -> Self {
        self.cfg.journald = enable;
        self
    }

    /// Enables ANSI color codes in log output.
    pub fn colored(mut self, enable: bool) -> Self {
        self.cfg.colored = enable;
        self
    }

    /// Includes timestamps with each log line.
    pub fn timestamps(mut self, enable: bool) -> Self {
        self.cfg.timestamps = enable;
        self
    }

    /// Preserves file permission bits during synchronization.
    pub fn perms(mut self, enable: bool) -> Self {
        self.cfg.perms = enable;
        self
    }

    /// Preserves modification times on synchronized files.
    pub fn times(mut self, enable: bool) -> Self {
        self.cfg.times = enable;
        self
    }

    /// Preserves access times on synchronized files.
    pub fn atimes(mut self, enable: bool) -> Self {
        self.cfg.atimes = enable;
        self
    }

    /// Copies symbolic links as links rather than following them.
    pub fn links(mut self, enable: bool) -> Self {
        self.cfg.links = enable;
        self
    }

    /// Preserves device files.
    pub fn devices(mut self, enable: bool) -> Self {
        self.cfg.devices = enable;
        self
    }

    /// Preserves special files such as FIFOs.
    pub fn specials(mut self, enable: bool) -> Self {
        self.cfg.specials = enable;
        self
    }

    /// Preserves file ownership information.
    pub fn owner(mut self, enable: bool) -> Self {
        self.cfg.owner = enable;
        self
    }

    /// Preserves group ownership information.
    pub fn group(mut self, enable: bool) -> Self {
        self.cfg.group = enable;
        self
    }

    /// Preserves extended attributes.
    ///
    /// Requires the `xattr` feature.
    #[cfg(feature = "xattr")]
    pub fn xattrs(mut self, enable: bool) -> Self {
        self.cfg.xattrs = enable;
        self
    }

    /// Preserves POSIX access control lists.
    ///
    /// Requires the `acl` feature.
    #[cfg(feature = "acl")]
    pub fn acls(mut self, enable: bool) -> Self {
        self.cfg.acls = enable;
        self
    }

    /// Finalizes the builder and returns the constructed [`SyncConfig`].
    pub fn build(self) -> SyncConfig {
        self.cfg
    }
}

pub fn synchronize_with_config<P: AsRef<Path>>(src: P, dst: P, cfg: &SyncConfig) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let sub_cfg = SubscriberConfig::builder()
        .format(cfg.log_format)
        .verbose(cfg.verbose)
        .info(cfg.info.clone())
        .debug(cfg.debug.clone())
        .quiet(cfg.quiet)
        .log_file(cfg.log_file.clone())
        .syslog(cfg.syslog)
        .journald(cfg.journald)
        .colored(cfg.colored)
        .timestamps(cfg.timestamps)
        .build();
    let sub = subscriber(sub_cfg);
    with_default(sub, || -> Result<()> {
        let opts = SyncOptions {
            perms: cfg.perms,
            times: cfg.times,
            atimes: cfg.atimes,
            links: cfg.links,
            devices: cfg.devices,
            specials: cfg.specials,
            owner: cfg.owner,
            group: cfg.group,
            quiet: cfg.quiet,
            #[cfg(feature = "xattr")]
            xattrs: cfg.xattrs,
            #[cfg(feature = "acl")]
            acls: cfg.acls,
            ..SyncOptions::default()
        };
        engine::sync(src, dst, &Matcher::default(), &available_codecs(), &opts)?;
        Ok(())
    })
}

pub fn synchronize<P: AsRef<Path>>(src: P, dst: P) -> Result<()> {
    synchronize_with_config(src, dst, &SyncConfig::default())
}
