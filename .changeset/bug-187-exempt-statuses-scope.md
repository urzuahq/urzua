---
default: patch
---

`config.scope-matches-nothing` no longer warns about records a `sections.exempt_statuses` declaration sets aside:
`section.required` and `section.shape` count them as eligible and unexamined instead of out of scope, so a
declared exemption is not reported as a misconfiguration.
