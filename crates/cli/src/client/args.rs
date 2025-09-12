// crates/cli/src/client/args.rs

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use clap::ArgMatches;
use engine::Result;
use oc_rsync_core::filter::{self, Matcher, Rule, default_cvs_rules};

use crate::EngineError;
use crate::utils::parse_filters;

use crate::options::ClientOpts;

pub(crate) fn build_matcher(opts: &ClientOpts, matches: &ArgMatches) -> Result<Matcher> {
    let mut entries: Vec<(usize, usize, Rule)> = Vec::new();
    let mut seq = 0;
    let mut add_rules = |idx: usize, rs: Vec<Rule>| {
        for r in rs {
            entries.push((idx, seq, r));
            seq += 1;
        }
    };

    if let Some(values) = matches.get_many::<String>("filter") {
        let idxs: Vec<_> = matches
            .indices_of("filter")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, val) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(val, opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("filter_file") {
        let idxs: Vec<_> = matches
            .indices_of("filter_file")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let rs = if file == Path::new("-") {
                if opts.from0 {
                    filter::parse_file(Path::new("-"), true, &mut HashSet::new(), 0)
                        .map_err(|e| EngineError::Other(format!("{:?}", e)))?
                } else {
                    return Err(EngineError::Other(
                        "filter file '-' requires --from0".into(),
                    ));
                }
            } else {
                filter::parse_file(file, opts.from0, &mut HashSet::new(), 0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?
            };
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<String>("include") {
        let idxs: Vec<_> = matches
            .indices_of("include")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let mut bytes = pat.clone().into_bytes();
            if opts.from0 {
                bytes.push(0);
            } else {
                bytes.push(b'\n');
            }
            let rs =
                filter::parse_rule_list_from_bytes(&bytes, opts.from0, '+', &mut vset, 0, None)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<String>("exclude") {
        let idxs: Vec<_> = matches
            .indices_of("exclude")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, pat) in idxs.into_iter().zip(values) {
            add_rules(
                idx + 1,
                parse_filters(&format!("- {}", pat), opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("include_from") {
        let idxs: Vec<_> = matches
            .indices_of("include_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filter::parse_rule_list_file(file, opts.from0, '+', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("exclude_from") {
        let idxs: Vec<_> = matches
            .indices_of("exclude_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filter::parse_rule_list_file(file, opts.from0, '-', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if let Some(values) = matches.get_many::<PathBuf>("files_from") {
        let idxs: Vec<_> = matches
            .indices_of("files_from")
            .map_or_else(Vec::new, |v| v.collect());
        for (idx, file) in idxs.into_iter().zip(values) {
            let mut vset = HashSet::new();
            let rs = filter::parse_rule_list_file(file, opts.from0, '+', &mut vset, 0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?;
            add_rules(idx + 1, rs);
        }
    }
    if matches.contains_id("filter_shorthand") {
        if let Some(idx) = matches.index_of("filter_shorthand") {
            let count = matches.get_count("filter_shorthand");
            let rule_str = if count >= 2 { "-FF" } else { "-F" };
            add_rules(
                idx + 1,
                parse_filters(rule_str, opts.from0)
                    .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
            );
        }
    }
    if !opts.files_from.is_empty() {
        add_rules(
            usize::MAX,
            parse_filters("- /**", opts.from0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
    }
    if opts.cvs_exclude {
        let mut cvs_rules =
            default_cvs_rules().map_err(|e| EngineError::Other(format!("{:?}", e)))?;
        cvs_rules.extend(
            parse_filters(":C\n", opts.from0)
                .map_err(|e| EngineError::Other(format!("{:?}", e)))?,
        );
        add_rules(usize::MAX, cvs_rules);
    }

    entries.sort_by(|a, b| {
        if a.0 == b.0 {
            a.1.cmp(&b.1)
        } else {
            a.0.cmp(&b.0)
        }
    });
    let rules: Vec<Rule> = entries.into_iter().map(|(_, _, r)| r).collect();
    let mut matcher = Matcher::new(rules);
    if opts.from0 {
        matcher = matcher.with_from0();
    }
    if opts.existing {
        matcher = matcher.with_existing();
    }
    if opts.prune_empty_dirs {
        matcher = matcher.with_prune_empty_dirs();
    }
    if opts.no_implied_dirs {
        matcher = matcher.with_no_implied_dirs();
    }
    Ok(matcher)
}
