---
default: minor
closes: []
---

A new opt-in `check` rule, `section.decided-body-changed`, reports a change to the body of a decided record other than an added section matching a declared `appendable_sections` pattern (such as a dated `## Amendment (YYYY-MM-DD)`) or rows added to the end of its revision log, so a decision is amended and never rewritten. It runs on a `--base` run against the merge base's copy, compares words so a formatter's re-wrapping or re-padded tables are not changes, and reads which types and statuses are decided from `decision_types`. It is off unless declared and says `not-applicable` on a run with no `--base`.
