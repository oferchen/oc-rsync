// crates/cli/src/exec/privileges.rs

use clap::{ArgMatches, parser::ValueSource};

use crate::EngineError;
use crate::options::ClientOpts;
use oc_rsync_core::{message::ExitCode, transfer::Result};

#[cfg(unix)]
use nix::unistd;

#[cfg(target_os = "linux")]
use caps::{self, CapSet, Capability};

pub(crate) fn check_privileges(opts: &mut ClientOpts, matches: &ArgMatches) -> Result<()> {
    #[cfg(unix)]
    {
        let need_owner = if opts.no_owner {
            false
        } else {
            opts.owner || opts.archive
        };
        let need_group = if opts.no_group {
            false
        } else {
            opts.group || opts.archive
        };
        let maps_requested =
            opts.chown.is_some() || !opts.usermap.is_empty() || !opts.groupmap.is_empty();
        let needs_privs = need_owner || need_group || maps_requested;
        let numeric_fallback = opts.numeric_ids
            && opts.chown.is_none()
            && opts.usermap.is_empty()
            && opts.groupmap.is_empty();
        if needs_privs && !numeric_fallback && !is_effective_root() {
            #[cfg(target_os = "linux")]
            let has_privs = match has_cap_chown() {
                Ok(v) => v,
                Err(e) => {
                    return Err(EngineError::Other(format!(
                        "failed to detect CAP_CHOWN capability: {e}",
                    )));
                }
            };
            #[cfg(not(target_os = "linux"))]
            let has_privs = false;

            let priv_msg = if cfg!(target_os = "linux") {
                "changing ownership requires root or CAP_CHOWN"
            } else {
                "changing ownership requires root"
            };

            if !has_privs {
                if maps_requested {
                    return Err(EngineError::Exit(ExitCode::StartClient, priv_msg.into()));
                }
                let owner_explicit =
                    matches.value_source("owner") == Some(ValueSource::CommandLine);
                let group_explicit =
                    matches.value_source("group") == Some(ValueSource::CommandLine);
                let mut downgraded = false;
                if need_owner && !owner_explicit {
                    opts.owner = false;
                    opts.no_owner = true;
                    downgraded = true;
                }
                if need_group && !group_explicit {
                    opts.group = false;
                    opts.no_group = true;
                    downgraded = true;
                }
                if downgraded {
                    tracing::warn!("{priv_msg}: disabling owner/group");
                } else {
                    return Err(EngineError::Exit(ExitCode::StartClient, priv_msg.into()));
                }
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn is_effective_root() -> bool {
    #[cfg(test)]
    if let Some(v) = MOCK_IS_ROOT.with(|m| m.borrow_mut().take()) {
        return v;
    }
    unistd::Uid::effective().is_root()
}

#[cfg(all(test, unix))]
thread_local! {
    static MOCK_IS_ROOT: std::cell::RefCell<Option<bool>> = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, unix))]
fn mock_effective_root(val: bool) {
    MOCK_IS_ROOT.with(|m| *m.borrow_mut() = Some(val));
}

#[cfg(target_os = "linux")]
fn has_cap_chown() -> std::result::Result<bool, caps::errors::CapsError> {
    #[cfg(test)]
    if let Some(res) = MOCK_CAPS.with(|m| m.borrow_mut().take()) {
        return res;
    }
    caps::has_cap(None, CapSet::Effective, Capability::CAP_CHOWN)
}

#[cfg(all(test, target_os = "linux"))]
thread_local! {
    static MOCK_CAPS: std::cell::RefCell<
        Option<std::result::Result<bool, caps::errors::CapsError>>,
    > = const { std::cell::RefCell::new(None) };
}

#[cfg(all(test, target_os = "linux"))]
fn mock_caps_has_cap(res: std::result::Result<bool, caps::errors::CapsError>) {
    MOCK_CAPS.with(|m| *m.borrow_mut() = Some(res));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::cli_command;
    use clap::FromArgMatches;

    #[test]
    fn privilege_downgrade_without_root() {
        let cmd = cli_command();
        let matches = cmd
            .clone()
            .try_get_matches_from(["prog", "src", "dst"])
            .unwrap();
        let mut opts = ClientOpts::from_arg_matches(&matches).unwrap();
        opts.owner = true;
        opts.no_owner = false;
        opts.group = false;
        opts.no_group = true;
        #[cfg(unix)]
        {
            mock_effective_root(false);
            #[cfg(target_os = "linux")]
            mock_caps_has_cap(Ok(false));
        }
        check_privileges(&mut opts, &matches).unwrap();
        assert!(!opts.owner);
        assert!(opts.no_owner);
    }

    #[test]
    fn privilege_error_when_mapping() {
        let cmd = cli_command();
        let matches = cmd
            .clone()
            .try_get_matches_from(["prog", "src", "dst"])
            .unwrap();
        let mut opts = ClientOpts::from_arg_matches(&matches).unwrap();
        opts.chown = Some("0:0".into());
        #[cfg(unix)]
        {
            mock_effective_root(false);
            #[cfg(target_os = "linux")]
            mock_caps_has_cap(Ok(false));
        }
        let err = check_privileges(&mut opts, &matches).unwrap_err();
        matches!(err, EngineError::Exit(ExitCode::StartClient, _));
    }
}
