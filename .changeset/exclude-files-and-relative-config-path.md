---
default: minor
closes: [BUG-183, BUG-185]
---

Add a per-type `exclude_files` declaration to `.urzua/config.yaml`: bare file names directly in a
type's `dir` that are not its records, such as an index `README.md`. A named file is never loaded as
a record, so no rule examines it and `filename.identity-pattern` no longer suggests renaming it or
counts it among the type's files; any other stray file is still reported. An entry that is a path, a
glob or empty is refused when the configuration loads. Findings about the configuration, such as
`config.scope-matches-nothing`, now name `.urzua/config.yaml` relative to the repository root
instead of an absolute path, so a report compares across checkouts.
