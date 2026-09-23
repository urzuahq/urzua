---
default: patch
---

`SPEC-5` (`urzua init`) documented a full greenfield mode, `--types`/`--dir` flags, and built-in
profiles as if built -- none of it exists; `init` has one mode (adopt an existing corpus) and two
flags (`--dry-run`, `--config`). That aspirational design is relocated to a new milestone, `MILE-114`,
and `SPEC-5` is corrected to describe only the adopt path that actually ships, now `Accepted`. Two more
inaccuracies found in the same pass and corrected: the Layout section implied `init` writes
`templates/`, `cache/`, and `.gitignore`; it writes only `.urzua/config.yaml` today. Documentation
only; no behavior change.
