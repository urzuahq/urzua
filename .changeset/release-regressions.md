---
default: patch
---

Four defects found by reviewing the release diff as a whole, two of them
regressions introduced by this release's own fixes.

A date-named file no longer hijacks `urzua new`'s numbering (`BUG-53`). Sharing
one filename recogniser between `init` and `new` dropped a length constraint, so
`2026-09-19-notes.md` parsed as record 2026 and the corpus was stuck above it
permanently, since numbers are never reused.

One stray record-shaped file no longer collapses every sibling record type into
its parent (`BUG-54`). A corpus with `docs/adr/`, `docs/rfc/` and a single
`docs/0099-index.md` proposed one meaningless `doc` type and ingested
`docs/README.md`, which is not a record.

`embodiment.locator-exists` says "is not a git-tracked file" rather than "is not
in the working tree" (`BUG-55`), which was false of a gitignored file.

A `claim_paths` entry that is not a readable directory now fails at startup
instead of silently disabling the rule (`BUG-56`).

`README.md` and `AGENTS.md` no longer instruct readers to edit
`.urzua/config.toml`, which this release stops reading.
