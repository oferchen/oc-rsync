# fuzz/src/lib.rs

Shared helper utilities for fuzzing targets.


Utilities that are reused across fuzz targets.


Keeping the helpers in a separate module makes it easy for each

fuzz target to pull in the small bits of functionality it needs

without repeating boilerplate.
