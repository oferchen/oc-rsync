// src/run.rs

use compress::available_codecs;
use engine::{Result, SyncOptions};
use filters::Matcher;
use tracing::subscriber::with_default;

use crate::config::SyncConfig;
use logging::{SubscriberConfig, subscriber};
use std::path::Path;

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
