---
Stable-Id: 01M33XB9G6FBCM194E0PKP97YD
Status: Planned
Phase: '0'
Track: schema-governance
Implements: RFC-41
Blocked-on: —
---
# 108 — build code.dangling-bug-reference, a rule checking source comments cite real records

## What

`RFC-41`'s implementation: a new rule, `code.dangling-bug-reference`, that reads tracked source files
(extensions declared in config, per `ADR-53`) for comments citing a record id (`BUG-\d+`, `ADR-\d+`,
`RFC-\d+`, matching this project's own type prefixes) and reports one whose cited record does not
exist under that type's declared `dir`. The mirror of `embodiment.locator-exists`, which already checks
the opposite direction — a record's locator naming a real file.

## Why

Phase 0: this is a gap in the engine's own governance, found on this project's own corpus, the same
day it was found. Round 15's `BUG-104` → `BUG-108` renumbering left two source comments in `record.rs`
and `init.rs` citing the deleted `BUG-104`; nothing caught it until a human ran `grep -rn "BUG-104"` by
hand after a `/code-review` pass separately flagged one of the two instances for an unrelated reason
(temporal language) and the fix happened to notice the other while correcting it.

`RFC-41` names the open questions (comment-syntax scope, string-literal vs. doc-comment citations,
distinguishing "dangling" from "superseded," whether non-record `.md` prose is in scope) that this
milestone's implementation has to resolve, in an ADR, before shipping the rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed at Phase 0. **Why:** the incident that motivated `RFC-41` happened during this same review round, and this project's own precedent (`MILE-107`'s filing after `RFC-40`/`RFC-39`) is to schedule the work while the evidence is fresh rather than leave the RFC as an unscheduled proposal. | **substantive** |
