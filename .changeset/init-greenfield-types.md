---
default: minor
closes: [MILE-114]
---

`urzua init` now sets up a repository that has no records yet. `urzua init --types adr,rfc,spec,prd`
picks the record types to govern from built-in profiles, and `--dir` sets the layout root (default
`docs`, so each type lives in `docs/<type>`). The profile for each type is written out into
`.urzua/config.yaml` as ordinary, editable rules: its required fields, status values and allowed status
changes. A starter template for each type goes in `.urzua/templates/`. On a terminal, `urzua init` in a
repository with no record directory asks which types to govern. Running it again with the same
selection changes nothing and exits 0.

Record-shaped files that no selected type governs are reported in a notice, never as an error, and
adopt now names a record sitting at the repository root, which it cannot assign to a type.
