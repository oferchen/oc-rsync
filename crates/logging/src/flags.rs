// crates/logging/src/flags.rs
#![allow(missing_docs)]

use clap::ValueEnum;
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum LogFormat {
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum InfoFlag {
    Backup,
    Copy,
    Del,
    Flist,
    Flist2,
    Misc,
    Misc2,
    Mount,
    Name,
    Name2,
    Nonreg,
    Progress,
    Progress2,
    Remove,
    Skip,
    Skip2,
    Stats,
    Stats2,
    Stats3,
    Symsafe,
    Filter,
}

impl InfoFlag {
    pub const fn as_str(self) -> &'static str {
        match self {
            InfoFlag::Backup => "backup",
            InfoFlag::Copy => "copy",
            InfoFlag::Del => "del",
            InfoFlag::Flist => "flist",
            InfoFlag::Flist2 => "flist2",
            InfoFlag::Misc => "misc",
            InfoFlag::Misc2 => "misc2",
            InfoFlag::Mount => "mount",
            InfoFlag::Name => "name",
            InfoFlag::Name2 => "name2",
            InfoFlag::Nonreg => "nonreg",
            InfoFlag::Progress => "progress",
            InfoFlag::Progress2 => "progress2",
            InfoFlag::Remove => "remove",
            InfoFlag::Skip => "skip",
            InfoFlag::Skip2 => "skip2",
            InfoFlag::Stats => "stats",
            InfoFlag::Stats2 => "stats2",
            InfoFlag::Stats3 => "stats3",
            InfoFlag::Symsafe => "symsafe",
            InfoFlag::Filter => "filter",
        }
    }

    pub const fn target(self) -> &'static str {
        match self {
            InfoFlag::Backup => "info::backup",
            InfoFlag::Copy => "info::copy",
            InfoFlag::Del => "info::del",
            InfoFlag::Flist | InfoFlag::Flist2 => "info::flist",
            InfoFlag::Misc | InfoFlag::Misc2 => "info::misc",
            InfoFlag::Mount => "info::mount",
            InfoFlag::Name | InfoFlag::Name2 => "info::name",
            InfoFlag::Nonreg => "info::nonreg",
            InfoFlag::Progress | InfoFlag::Progress2 => "info::progress",
            InfoFlag::Remove => "info::remove",
            InfoFlag::Skip | InfoFlag::Skip2 => "info::skip",
            InfoFlag::Stats | InfoFlag::Stats2 | InfoFlag::Stats3 => "info::stats",
            InfoFlag::Symsafe => "info::symsafe",
            InfoFlag::Filter => "info::filter",
        }
    }
}

impl From<&InfoFlag> for InfoFlag {
    fn from(flag: &InfoFlag) -> Self {
        *flag
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, ValueEnum)]
#[clap(rename_all = "kebab-case")]
pub enum DebugFlag {
    Acl,
    Backup,
    Bind,
    Chdir,
    Connect,
    Cmd,
    Del,
    Deltasum,
    Dup,
    Exit,
    Filter,
    Flist,
    Fuzzy,
    Genr,
    Hash,
    Hlink,
    Iconv,
    Io,
    Nstr,
    Own,
    Proto,
    Recv,
    Send,
    Time,
}

impl DebugFlag {
    pub const fn as_str(self) -> &'static str {
        match self {
            DebugFlag::Acl => "acl",
            DebugFlag::Backup => "backup",
            DebugFlag::Bind => "bind",
            DebugFlag::Chdir => "chdir",
            DebugFlag::Connect => "connect",
            DebugFlag::Cmd => "cmd",
            DebugFlag::Del => "del",
            DebugFlag::Deltasum => "deltasum",
            DebugFlag::Dup => "dup",
            DebugFlag::Exit => "exit",
            DebugFlag::Filter => "filter",
            DebugFlag::Flist => "flist",
            DebugFlag::Fuzzy => "fuzzy",
            DebugFlag::Genr => "genr",
            DebugFlag::Hash => "hash",
            DebugFlag::Hlink => "hlink",
            DebugFlag::Iconv => "iconv",
            DebugFlag::Io => "io",
            DebugFlag::Nstr => "nstr",
            DebugFlag::Own => "own",
            DebugFlag::Proto => "proto",
            DebugFlag::Recv => "recv",
            DebugFlag::Send => "send",
            DebugFlag::Time => "time",
        }
    }

    pub const fn target(self) -> &'static str {
        match self {
            DebugFlag::Acl => "debug::acl",
            DebugFlag::Backup => "debug::backup",
            DebugFlag::Bind => "debug::bind",
            DebugFlag::Chdir => "debug::chdir",
            DebugFlag::Connect => "debug::connect",
            DebugFlag::Cmd => "debug::cmd",
            DebugFlag::Del => "debug::del",
            DebugFlag::Deltasum => "debug::deltasum",
            DebugFlag::Dup => "debug::dup",
            DebugFlag::Exit => "debug::exit",
            DebugFlag::Filter => "debug::filter",
            DebugFlag::Flist => "debug::flist",
            DebugFlag::Fuzzy => "debug::fuzzy",
            DebugFlag::Genr => "debug::genr",
            DebugFlag::Hash => "debug::hash",
            DebugFlag::Hlink => "debug::hlink",
            DebugFlag::Iconv => "debug::iconv",
            DebugFlag::Io => "debug::io",
            DebugFlag::Nstr => "debug::nstr",
            DebugFlag::Own => "debug::own",
            DebugFlag::Proto => "debug::proto",
            DebugFlag::Recv => "debug::recv",
            DebugFlag::Send => "debug::send",
            DebugFlag::Time => "debug::time",
        }
    }
}

impl From<&DebugFlag> for DebugFlag {
    fn from(flag: &DebugFlag) -> Self {
        *flag
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum, Default)]
#[clap(rename_all = "kebab-case")]
pub enum StderrMode {
    #[clap(alias = "e")]
    #[default]
    Errors,
    #[clap(alias = "a")]
    All,
    #[clap(alias = "c")]
    Client,
}

#[derive(Clone, Debug)]
pub struct SubscriberConfig {
    pub format: LogFormat,
    pub verbose: u8,
    pub info: Vec<InfoFlag>,
    pub debug: Vec<DebugFlag>,
    pub quiet: bool,
    pub stderr: StderrMode,
    pub log_file: Option<(PathBuf, Option<String>)>,
    pub syslog: bool,
    pub journald: bool,
    pub colored: bool,
    pub timestamps: bool,
}

impl Default for SubscriberConfig {
    fn default() -> Self {
        Self {
            format: LogFormat::Text,
            verbose: 0,
            info: Vec::new(),
            debug: Vec::new(),
            quiet: false,
            stderr: StderrMode::Errors,
            log_file: None,
            syslog: false,
            journald: false,
            colored: true,
            timestamps: false,
        }
    }
}

#[derive(Default)]
pub struct SubscriberConfigBuilder {
    cfg: SubscriberConfig,
}

impl SubscriberConfig {
    pub fn builder() -> SubscriberConfigBuilder {
        SubscriberConfigBuilder::default()
    }
}

impl SubscriberConfigBuilder {
    pub fn format(mut self, format: LogFormat) -> Self {
        self.cfg.format = format;
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

    pub fn stderr(mut self, stderr: StderrMode) -> Self {
        self.cfg.stderr = stderr;
        self
    }

    pub fn log_file(mut self, log_file: Option<(PathBuf, Option<String>)>) -> Self {
        self.cfg.log_file = log_file;
        self
    }

    pub fn syslog(mut self, syslog: bool) -> Self {
        self.cfg.syslog = syslog;
        self
    }

    pub fn journald(mut self, journald: bool) -> Self {
        self.cfg.journald = journald;
        self
    }

    pub fn colored(mut self, colored: bool) -> Self {
        self.cfg.colored = colored;
        self
    }

    pub fn timestamps(mut self, timestamps: bool) -> Self {
        self.cfg.timestamps = timestamps;
        self
    }

    pub fn build(self) -> SubscriberConfig {
        self.cfg
    }
}
