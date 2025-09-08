# API Mapping

These tables document module relocations from recent refactors. Public items retain their original paths via re-exports, as verified with `cargo public-api`.

## Engine crate

| Original path | New module path |
| --- | --- |
| `engine::DeleteMode` | `engine::session::DeleteMode` |
| `engine::IdMapper` | `engine::session::IdMapper` |
| `engine::Stats` | `engine::session::Stats` |
| `engine::SyncOptions` | `engine::session::SyncOptions` |
| `engine::pipe_sessions` | `engine::session::pipe_sessions` |
| `engine::select_codec` | `engine::session::select_codec` |
| `engine::sync` | `engine::session::sync` |
| `engine::Receiver` | `engine::receiver::Receiver` |
| `engine::ReceiverState` | `engine::receiver::ReceiverState` |
| `engine::Sender` | `engine::sender::Sender` |
| `engine::SenderState` | `engine::sender::SenderState` |

## Filters crate

| Original path | New module path |
| --- | --- |
| `filters::MatchResult` | `filters::matcher::MatchResult` |
| `filters::Matcher` | `filters::matcher::Matcher` |
| `filters::RuleFlags` | `filters::rule::RuleFlags` |
| `filters::RuleData` | `filters::rule::RuleData` |
| `filters::Rule` | `filters::rule::Rule` |
| `filters::PerDir` | `filters::perdir::PerDir` |
| `filters::FilterStats` | `filters::stats::FilterStats` |
| `filters::parse_with_options` | `filters::parser::parse_with_options` |
| `filters::parse` | `filters::parser::parse` |
| `filters::default_cvs_rules` | `filters::parser::default_cvs_rules` |
| `filters::parse_list` | `filters::parser::parse_list` |
| `filters::parse_list_file` | `filters::parser::parse_list_file` |
| `filters::parse_from_bytes` | `filters::parser::parse_from_bytes` |
| `filters::parse_file` | `filters::parser::parse_file` |
| `filters::rooted_and_parents` | `filters::parser::rooted_and_parents` |
| `filters::parse_rule_list_from_bytes` | `filters::parser::parse_rule_list_from_bytes` |
| `filters::parse_rule_list_file` | `filters::parser::parse_rule_list_file` |

