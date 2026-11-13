---
default: minor
closes: []
---

New command `urzua waive --from-report`. It runs `check`, then previews one draft waiver per rule and
file for the findings that are unwaived and at or above `fail_on`, each with a pending `Reason` and no
`Expires`, so a draft suppresses nothing until a person fills it in. It prints one JSON document:
the drafts, the findings it skipped with the reason (an existing waiver, an expired one, a finding the
drafts themselves resolve), and any `--rule` or `--path` filter that matched nothing. It refuses
(exit 2) when a draft could take effect: the waiver type must require `Reason`, and must either declare
`permanent` or require `Expires`. This release only previews and writes no file.
