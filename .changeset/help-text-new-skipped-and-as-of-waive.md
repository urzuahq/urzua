---
default: patch
closes: []
---

Correct two lines of `--help`. `urzua new --help` said the numbers it skips are named on stderr; they
are listed in the `skipped` field of its JSON, not on stderr. The `--as-of` help
omitted `waive`, which accepts it in its read-only preview.
