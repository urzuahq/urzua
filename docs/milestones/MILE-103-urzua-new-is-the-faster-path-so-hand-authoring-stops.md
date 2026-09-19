---
Status: Planned
Stable-Id: 01M2W199SQBR59QQ77A6FR11K5
Phase: '0'
Track: governance-process
Implements: SPEC-1
Blocked-on: —
---
# 103 — `urzua new` is the faster path, so hand-authoring stops

## What

`SPEC-1`'s fourth success criterion:

> `urzua new` is what people and agents actually invoke, because hand-authoring is now the slower path.

## Why it is not satisfied by `urzua new` existing

It does exist (`ADR-27`) and is used in this repository. The criterion is about **preference under
choice**, not availability -- it is met when hand-authoring is slower, not when the command is
present.

Evidence it is not met yet: several records in this corpus were hand-written during this project's own
work, and two of them were rejected by `check` for header shapes `urzua new` would have produced
correctly. The command was available and not reached for.

## What would satisfy it

A measurement rather than an assertion. Candidates, unchosen:

- **Every record created in a window came from `urzua new`**, checkable from git history against the
  synthesized-header shape the command emits.
- **`urzua new` covers the cases people currently hand-write around** -- it refuses rather than
  guessing when neither a template nor `required_fields` applies (`ADR-27`), and each refusal is
  someone about to hand-author.

The second is the useful one, because a refusal is a recorded instance of the criterion failing.

## Not blocked

Independent of `MILE-98` and `MILE-100`. It is about the authoring path in whatever corpus is in
front of the tool.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s criterion 4 had no milestone and was deleted from the spec alongside criterion 3. It is about preference under choice, not availability -- `urzua new` exists and records in this corpus were still hand-written, two of them rejected for header shapes the command would have produced correctly. | **substantive** |
