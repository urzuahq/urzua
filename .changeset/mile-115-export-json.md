---
default: minor
closes: []
---

# Add `urzua export --format json`

Emits every record of every configured type as plain JSON on stdout: its type, path, display id
(e.g. `MILE-115`), `Stable-Id` when it has one, the header fields as `check` parses them, and the
file's full text as `raw_text`. Tooling can read records through `urzua` instead of a second
parser. `--format json` must be named explicitly; `--format agdr` is unchanged and still not
implemented.

A record of a `header_shape: none` type is reported as `header_readable: true`. A tracked record
whose real path is outside the repository (a symlink) stops the export with an error instead of
having its contents published.
