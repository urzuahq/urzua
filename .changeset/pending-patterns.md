---
default: minor
closes: []
---

`statuses.pending` accepts a pattern entry, `{pattern: "^\\(pending"}`, beside whole values, so a family of
placeholder statuses that share a stem is declared once. A value matching no entry and no pattern is still
a real value, and an invalid, blank or repeated pattern is a load error naming the type.
