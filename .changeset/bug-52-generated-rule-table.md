---
default: minor
---

Fixes `BUG-52`: `SPEC-2`'s rule table claimed to be "the complete, current set" while nothing checked
that against `ALL_RULES`, and it had drifted twice already. The table is now generated, not
hand-maintained — `urzua_core::rules::RULE_METADATA` (checked against `ALL_RULES` by a compiled test)
is the single source, rendered into `SPEC-2` by `scripts/generate-rule-table.py` between marker
comments. `make rule-table-check` (wired into `make ci`) fails the build if the committed table is
stale.

New: `urzua rules` — lists the complete, current rule set this build ships, with each rule's
description, reading no config or corpus. What the binary supports, not what any one repository
enabled.

No adopter-facing behavior change to `check`/`audit`/etc.: verified with the full test suite, clippy,
`make ci`, and a real-corpus `check` run reporting the same 70 findings before and after.
