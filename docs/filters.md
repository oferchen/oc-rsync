# Filter Engine

The filter engine mirrors rsync's include/exclude syntax and now supports
additional rule actions:

- `merge FILE` – load additional rules from `FILE` at parse time.
- `dir-merge FILE` – load per-directory rules from `FILE` as rsync scans
  directories.
- `hide PATTERN` – alias for `exclude`; hides matches from the transfer.
- `show PATTERN` – alias for `include`; reveals matches hidden by earlier rules.

Per-directory merge files are evaluated in the same order as upstream rsync.
Rules from deeper directories take precedence over ancestor directories and
their relative order is preserved with global rules.

The parser is fuzzed and property tested against rsync to ensure identical
semantics.
