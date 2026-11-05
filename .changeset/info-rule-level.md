---
default: minor
closes: [MILE-194]
---

A rule can now be set to `level: info`. Its findings are reported and counted like a warning's, with
`severity: "info"` in the report, but they never set `blocking` and never fail a run, whatever
`fail_on` says, so an advisory rule can be enabled in a repository that fails on warnings. `info` is
also accepted for a `field_levels` entry and a boundary `severity`, and a rule set to `info` never
reports above `info` because a field level or a boundary declared otherwise. `fail_on: info` is
refused, and every unrecognised level is refused at load with a message naming `off`, `info`,
`warn` and `error`.
