// crates/cli/src/argparse/builder.rs

use std::env;

#[cfg(any(test, feature = "dump-help"))]
use clap::{Arg, ArgAction};
use clap::{ArgMatches, Args, CommandFactory, FromArgMatches, parser::ValueSource};

use crate::EngineError;
use crate::formatter;
use oc_rsync_core::transfer::Result;

use super::{ClientOpts, ProbeOpts};

pub struct ClientOptsBuilder<'a> {
    matches: &'a ArgMatches,
}

impl<'a> ClientOptsBuilder<'a> {
    pub fn from_matches(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    pub fn build(self) -> Result<ClientOpts> {
        let mut opts = ClientOpts::from_arg_matches(self.matches)
            .map_err(|e| EngineError::Other(e.to_string()))?;
        if opts.no_D {
            opts.no_devices = true;
            opts.no_specials = true;
        }
        if !opts.old_args
            && self.matches.value_source("secluded_args") != Some(ValueSource::CommandLine)
        {
            if let Ok(val) = env::var("RSYNC_PROTECT_ARGS") {
                if val != "0" {
                    opts.secluded_args = true;
                }
            }
        }
        Ok(opts)
    }
}

pub struct ProbeOptsBuilder<'a> {
    matches: &'a ArgMatches,
}

impl<'a> ProbeOptsBuilder<'a> {
    pub fn from_matches(matches: &'a ArgMatches) -> Self {
        Self { matches }
    }

    pub fn build(self) -> Result<ProbeOpts> {
        ProbeOpts::from_arg_matches(self.matches).map_err(|e| EngineError::Other(e.to_string()))
    }
}

pub fn cli_command() -> clap::Command {
    let cmd = ProbeOpts::command();
    let cmd = ClientOpts::augment_args(cmd);
    #[cfg(any(test, feature = "dump-help"))]
    let cmd = cmd.arg(
        Arg::new("dump-help-body")
            .long("dump-help-body")
            .action(ArgAction::SetTrue)
            .hide(true)
            .exclusive(true),
    );
    formatter::apply(cmd)
}
