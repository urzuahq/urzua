---
default: minor
---

# Add `urzua export --format json`

Emits every record of every configured type as plain JSON on stdout: its type, path, display id
(e.g. `MILE-115`), `Stable-Id` when it has one, the header fields as `check` parses them, and the
file's full text as `raw_text`. Tooling can read records through `urzua` instead of a second
parser. `--format json` must be named explicitly; `--format agdr` is unchanged and still not
implemented.
