---
default: minor
---

Two opt-in rules judge the paths a `check --base <ref>` run changed. `diff.source-requires-record`
reports each changed path under a declared `source_paths` prefix when no governed record changed in the
same diff. `diff.source-and-records-mixed` warns when one diff changes both a governed record and a path
under `application_paths` (less `exclude_paths`). On a run with no `--base` either rule reports
`not-applicable` in `rules_executed` instead of passing.
