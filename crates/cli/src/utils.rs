// crates/cli/src/utils.rs

use std::collections::HashSet;
use std::env;
use std::ffi::OsString;
use std::path::PathBuf;
use std::time::Duration;

use clap::ArgMatches;
use encoding_rs::Encoding;
use filters::{parse_with_options, Rule};
use logging::{DebugFlag, InfoFlag, LogFormat, SubscriberConfig};
use meta::{parse_id_map, IdKind};
use protocol::CharsetConv;
use shell_words::split as shell_split;

use engine::{EngineError, IdMapper, Result};

pub fn print_version_if_requested<I>(args: I) -> bool
where
    I: IntoIterator<Item = OsString>,
{
    let mut show_version = false;
    let mut quiet = false;
    for arg in args {
        if arg == "--version" || arg == "-V" {
            show_version = true;
        } else if arg == "--quiet" || arg == "-q" {
            quiet = true;
        }
    }
    if show_version {
        if !quiet {
            println!("{}", crate::version::render_version_lines().join("\n"));
        }
        true
    } else {
        false
    }
}

pub(crate) fn parse_filters(
    s: &str,
    from0: bool,
) -> std::result::Result<Vec<Rule>, filters::ParseError> {
    let mut v = HashSet::new();
    parse_with_options(s, from0, &mut v, 0, None)
}

pub(crate) fn parse_duration(s: &str) -> std::result::Result<Duration, std::num::ParseIntError> {
    Ok(Duration::from_secs(s.parse()?))
}

pub(crate) fn parse_nonzero_duration(s: &str) -> std::result::Result<Duration, String> {
    let d = parse_duration(s).map_err(|e| e.to_string())?;
    if d.as_secs() == 0 {
        Err("value must be greater than 0".into())
    } else {
        Ok(d)
    }
}

const SIZE_SUFFIXES: &[(char, u32)] = &[('k', 10), ('m', 20), ('g', 30)];

pub(crate) fn parse_suffixed<T>(s: &str, shifts: &[(char, u32)]) -> std::result::Result<T, String>
where
    T: TryFrom<u64>,
{
    let s = s.trim();
    if s == "0" {
        return T::try_from(0).map_err(|_| "size overflow".to_string());
    }
    if let Some(last) = s.chars().last() {
        if last.is_ascii_alphabetic() {
            let num = s[..s.len() - 1].parse::<u64>().map_err(|e| e.to_string())?;
            let shift = shifts
                .iter()
                .find(|(c, _)| last.eq_ignore_ascii_case(c))
                .map(|(_, s)| *s)
                .ok_or_else(|| format!("invalid size suffix: {last}"))?;
            let mult = 1u64 << shift;
            let val = num
                .checked_mul(mult)
                .ok_or_else(|| "size overflow".to_string())?;
            return T::try_from(val).map_err(|_| "size overflow".to_string());
        }
    }
    let val = s.parse::<u64>().map_err(|e| e.to_string())?;
    T::try_from(val).map_err(|_| "size overflow".to_string())
}

pub(crate) fn parse_size<T>(s: &str) -> std::result::Result<T, String>
where
    T: TryFrom<u64>,
{
    parse_suffixed(s, SIZE_SUFFIXES)
}

pub(crate) fn parse_dparam(s: &str) -> std::result::Result<(String, String), String> {
    let mut parts = s.splitn(2, '=');
    let name = parts
        .next()
        .ok_or_else(|| "invalid dparam".to_string())?
        .to_string();
    let value = parts
        .next()
        .ok_or_else(|| "invalid dparam".to_string())?
        .to_string();
    Ok((name, value))
}

pub(crate) fn parse_bool(s: &str) -> std::result::Result<bool, String> {
    if ["1", "true", "yes"]
        .iter()
        .any(|v| s.eq_ignore_ascii_case(v))
    {
        Ok(true)
    } else if ["0", "false", "no"]
        .iter()
        .any(|v| s.eq_ignore_ascii_case(v))
    {
        Ok(false)
    } else {
        Err("invalid boolean".to_string())
    }
}

pub(crate) fn parse_logging_flags(matches: &ArgMatches) -> (Vec<InfoFlag>, Vec<DebugFlag>) {
    let mut info: Vec<InfoFlag> = matches
        .get_many::<InfoFlag>("info")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    if matches.contains_id("out_format") && !info.contains(&InfoFlag::Name) {
        info.push(InfoFlag::Name);
    }
    let debug = matches
        .get_many::<DebugFlag>("debug")
        .map(|v| v.copied().collect())
        .unwrap_or_default();
    (info, debug)
}

pub(crate) fn init_logging(matches: &ArgMatches) {
    let verbose = matches.get_count("verbose");
    let quiet = matches.get_flag("quiet");
    let log_format = *matches
        .get_one::<LogFormat>("log_format")
        .unwrap_or(&LogFormat::Text);
    let log_file = matches.get_one::<PathBuf>("client-log-file").cloned();
    let log_file_fmt = matches.get_one::<String>("client-log-file-format").cloned();
    let syslog = matches.get_flag("syslog");
    let journald = matches.get_flag("journald");
    let (mut info, mut debug) = parse_logging_flags(matches);
    if quiet {
        info.clear();
        debug.clear();
    }
    let cfg = SubscriberConfig::builder()
        .format(log_format)
        .verbose(verbose)
        .info(info)
        .debug(debug)
        .quiet(quiet)
        .log_file(log_file.map(|p| (p, log_file_fmt)))
        .syslog(syslog)
        .journald(journald)
        .colored(true)
        .timestamps(false)
        .build();
    logging::init(cfg);
}

pub(crate) fn locale_charset() -> Option<String> {
    for var in ["LC_ALL", "LC_CTYPE", "LANG"] {
        if let Ok(val) = env::var(var) {
            if let Some(enc) = val.split('.').nth(1) {
                return Some(enc.to_string());
            }
        }
    }
    None
}

pub fn parse_iconv(spec: &str) -> std::result::Result<CharsetConv, String> {
    let mut parts = spec.split(',');
    let local_label = parts
        .next()
        .ok_or_else(|| "invalid iconv spec".to_string())?;
    let remote_label = parts.next().unwrap_or("UTF-8");

    let local_label = if local_label == "." {
        locale_charset().ok_or_else(|| "failed to determine locale charset".to_string())?
    } else {
        local_label.to_string()
    };
    let remote_label = if remote_label == "." {
        locale_charset().ok_or_else(|| "failed to determine locale charset".to_string())?
    } else {
        remote_label.to_string()
    };

    let local_enc = Encoding::for_label(local_label.as_bytes());
    let remote_enc = Encoding::for_label(remote_label.as_bytes());

    let local_enc = local_enc
        .ok_or_else(|| format!("iconv_open(\"{local_label}\", \"{remote_label}\") failed"))?;
    let remote_enc = remote_enc
        .ok_or_else(|| format!("iconv_open(\"{local_label}\", \"{remote_label}\") failed"))?;

    Ok(CharsetConv::new(remote_enc, local_enc))
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RshCommand {
    pub env: Vec<(String, String)>,
    pub cmd: Vec<String>,
}

fn parse_env_command(parts: Vec<String>) -> Result<RshCommand> {
    let mut env = Vec::new();
    let mut iter = parts.into_iter();
    let mut cmd = Vec::new();

    while let Some(tok) = iter.next() {
        if let Some((k, v)) = tok.split_once('=') {
            let valid = !k.is_empty()
                && (k.as_bytes()[0].is_ascii_alphabetic() || k.as_bytes()[0] == b'_')
                && k.as_bytes()[1..]
                    .iter()
                    .all(|b| b.is_ascii_alphanumeric() || *b == b'_');
            if valid {
                env.push((k.to_string(), v.to_string()));
                continue;
            } else {
                return Err(EngineError::Other("invalid environment assignment".into()));
            }
        }
        cmd.push(tok);
        cmd.extend(iter);
        return Ok(RshCommand { env, cmd });
    }
    Ok(RshCommand { env, cmd })
}

pub fn parse_rsh(raw: Option<String>) -> Result<RshCommand> {
    match raw {
        Some(s) => {
            let parts = shell_split(&s).map_err(|e| EngineError::Other(e.to_string()))?;
            let mut cmd = parse_env_command(parts)?;
            if cmd.cmd.is_empty() {
                cmd.cmd.push("ssh".to_string());
            }
            Ok(cmd)
        }
        None => Ok(RshCommand {
            env: Vec::new(),
            cmd: vec!["ssh".to_string()],
        }),
    }
}

pub fn parse_rsync_path(raw: Option<String>) -> Result<Option<RshCommand>> {
    match raw {
        Some(s) => {
            let parts = shell_split(&s).map_err(|e| EngineError::Other(e.to_string()))?;
            let cmd = parse_env_command(parts)?;
            if cmd.env.is_empty() && cmd.cmd.is_empty() {
                Ok(None)
            } else {
                Ok(Some(cmd))
            }
        }
        None => Ok(None),
    }
}

pub(crate) fn parse_name_map(specs: &[String], kind: IdKind) -> Result<Option<IdMapper>> {
    if specs.is_empty() {
        Ok(None)
    } else {
        let spec = specs.join(",");
        let mapper = parse_id_map(&spec, kind).map_err(EngineError::Other)?;
        Ok(Some(IdMapper(mapper)))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PathSpec {
    pub path: PathBuf,
    pub trailing_slash: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteSpec {
    Local(PathSpec),
    Remote {
        host: String,
        path: PathSpec,
        module: Option<String>,
    },
}

pub(crate) fn parse_remote_spec(input: &str) -> Result<RemoteSpec> {
    let (trailing_slash, s) = if input != "/" && input.ends_with('/') {
        (true, &input[..input.len() - 1])
    } else {
        (false, input)
    };
    if let Some(rest) = s.strip_prefix("rsync://") {
        let mut parts = rest.splitn(2, '/');
        let host = parts.next().unwrap_or("");
        let mod_path = parts.next().unwrap_or("");
        let mut mp = mod_path.splitn(2, '/');
        let module = mp.next().unwrap_or("");
        let path = mp.next().unwrap_or("");
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathSpec {
                path: PathBuf::from(path),
                trailing_slash,
            },
            module: Some(module.to_string()),
        });
    }
    if let Some(rest) = s.strip_prefix('[') {
        if let Some(end) = rest.find(']') {
            let host = &rest[..end];
            if let Some(path) = rest[end + 1..].strip_prefix(':') {
                return Ok(RemoteSpec::Remote {
                    host: host.to_string(),
                    path: PathSpec {
                        path: PathBuf::from(path),
                        trailing_slash,
                    },
                    module: None,
                });
            }
        }
        return Ok(RemoteSpec::Local(PathSpec {
            path: PathBuf::from(input),
            trailing_slash,
        }));
    }
    if let Some(idx) = s.find("::") {
        let host = &s[..idx];
        let mut rest = s[idx + 2..].splitn(2, '/');
        let module = rest.next().unwrap_or("");
        let path = rest.next().unwrap_or("");
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathSpec {
                path: PathBuf::from(path),
                trailing_slash,
            },
            module: Some(module.to_string()),
        });
    }
    if let Some(idx) = s.find(':') {
        if idx == 1 {
            let bytes = s.as_bytes();
            if bytes[0].is_ascii_alphabetic()
                && (bytes.len() == 2
                    || bytes
                        .get(2)
                        .map(|c| *c == b'/' || *c == b'\\')
                        .unwrap_or(false))
            {
                return Ok(RemoteSpec::Local(PathSpec {
                    path: PathBuf::from(s),
                    trailing_slash,
                }));
            }
        }
        let (host, path) = s.split_at(idx);
        return Ok(RemoteSpec::Remote {
            host: host.to_string(),
            path: PathSpec {
                path: PathBuf::from(&path[1..]),
                trailing_slash,
            },
            module: None,
        });
    }
    Ok(RemoteSpec::Local(PathSpec {
        path: PathBuf::from(s),
        trailing_slash,
    }))
}

pub(crate) fn parse_remote_specs(src: &str, dst: &str) -> Result<(RemoteSpec, RemoteSpec)> {
    let src_spec = parse_remote_spec(src)?;
    let dst_spec = parse_remote_spec(dst)?;
    if let (
        RemoteSpec::Remote {
            host: sh, path: sp, ..
        },
        RemoteSpec::Remote {
            host: dh, path: dp, ..
        },
    ) = (&src_spec, &dst_spec)
    {
        if sh.is_empty() || dh.is_empty() {
            return Err(EngineError::Other("remote host missing".into()));
        }
        if sp.path.as_os_str().is_empty() || dp.path.as_os_str().is_empty() {
            return Err(EngineError::Other("remote path missing".into()));
        }
    }
    Ok((src_spec, dst_spec))
}
