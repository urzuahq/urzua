---
"urzua": minor
---

Three defects found by reviewing the unreleased diff before cutting a release.

`urzua audit` bypassed the rules table entirely (`BUG-42`). `MILE-80` routed
every rule in `check` through the opt-in gate and left `audit` calling two rules
directly, so a repository declaring `pointer.resolution: off` was still blocked
by `audit`, and a declared level was ignored. The gate is now shared rather than
owned by one command.

`urzua init` dropped the outer record type when a subdirectory also held
records (`BUG-43`). A corpus with three records in `doc/adr/` and one in
`doc/adr/archive/` adopted only the archive, leaving three records ungoverned
with `check` reporting success over them. The outer directory now wins and
nested records are folded into it.

A rule declared without an option it requires is now a load-time error
(`BUG-44`). `claim.status-agreement` without `closed_statuses` treated every
claim as a violation, including correct ones; without `claim_paths` it scanned
nothing and reported success forever. `init` no longer proposes rules it cannot
declare completely.
