# Filter Engine

The filter engine mirrors rsync's include/exclude syntax and now supports
additional rule actions:

- `merge FILE` – load additional rules from `FILE` at parse time.
- `dir-merge FILE` – load per-directory rules from `FILE` as rsync scans
  directories.
- `hide PATTERN` – alias for `exclude`; hides matches from the transfer.
- `show PATTERN` – alias for `include`; reveals matches hidden by earlier rules.

Directory paths listed via `--files-from` are detected even without a trailing
slash, ensuring their contents are visited and any nested `.rsync-filter` rules
are merged during traversal.

Per-directory merge files are evaluated in the same order as upstream rsync.
Rules from deeper directories take precedence over ancestor directories and
their relative order is preserved with global rules.

The parser is fuzzed and property tested against rsync to ensure identical
semantics.

## Reporting

`Matcher` instances keep running statistics of rule evaluations. Each rule
records whether it matched or missed and the source file that defined it. After
filter checks, call `Matcher::stats()` or `Matcher::report()` to retrieve or log
these counters. `LoggingAgent` consumes this data on the `info::filter` target
to produce lines such as:

```
matches=1 misses=0 source=/tmp/rules
```
