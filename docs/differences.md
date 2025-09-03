# Known Differences

The following gaps remain between **oc-rsync** and upstream `rsync`. Each entry links to a tracking issue or reference location so it can be retired as parity work lands.

- **Hard link preservation** is not yet implemented ([#721](https://github.com/oferchen/oc-rsync/issues/721)).
- **Additional compression codecs** such as LZ4 are unsupported ([#873](https://github.com/oferchen/oc-rsync/issues/873)).
- The **`--open-noatime` flag** is parsed but ignored ([feature matrix](feature_matrix.md#--open-noatime)).
- The **`--no-D` alias** (`--no-devices --no-specials`) is missing ([feature matrix](feature_matrix.md#--no-d)).
- **Windows path and permission handling** is incomplete ([compatibility notes](compatibility.md#tested-platforms)).

