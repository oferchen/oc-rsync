// src/lib.rs

use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use logging::{DebugFlag, InfoFlag, LogFormat, SubscriberConfig, subscriber};
use std::path::{Path, PathBuf};
use tracing::subscriber::with_default;

pub use meta;

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
    pub fn builder() -> SyncConfigBuilder {
        SyncConfigBuilder::default()
    }
}

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
        self.cfg.info = info.into_iter().map(Into::into).collect();
        self
    }

    pub fn debug<I>(mut self, debug: I) -> Self
    where
        I: IntoIterator,
        I::Item: Into<DebugFlag>,
    {
        self.cfg.debug = debug.into_iter().map(Into::into).collect();
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

pub fn synchronize_with_config<S: AsRef<Path>, D: AsRef<Path>>(
    src: S,
    dst: D,
    cfg: &SyncConfig,
) -> Result<()> {
    let src = src.as_ref();
    let dst = dst.as_ref();
    let sub_cfg = SubscriberConfig::builder()
        .format(cfg.log_format)
        .verbose(cfg.verbose)
        .info(&cfg.info)
        .debug(&cfg.debug)
        .quiet(cfg.quiet)
        .log_file(cfg.log_file.clone())
        .syslog(cfg.syslog)
        .journald(cfg.journald)
        .colored(cfg.colored)
        .timestamps(cfg.timestamps)
        .build();
    let sub = subscriber(sub_cfg)?;
    with_default(sub, || -> Result<()> {
        let opts = SyncOptions {
            perms: cfg.perms,
            times: cfg.times,
            atimes: cfg.atimes,
            open_noatime: cfg.open_noatime,
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

pub fn synchronize<S: AsRef<Path>, D: AsRef<Path>>(src: S, dst: D) -> Result<()> {
    synchronize_with_config(src, dst, &SyncConfig::default())
}
