---
default: minor
---

Add the opt-in `citation.path-exists` rule. A path cited in backticks inside a declared reference
section (`reference_sections`, for example `["References"]`) must exist in the working tree: a file
git tracks and that is on disk, or a directory with a tracked file below it. Only plain paths are
read (a directory component and a file extension, or a trailing `/`); URLs, globs, placeholders and
bare file names are not. A trailing `:line` or `#anchor` is ignored and `./` or `../` resolves from
the record's directory. The check is offline and deterministic, and its population counts the paths
cited.
