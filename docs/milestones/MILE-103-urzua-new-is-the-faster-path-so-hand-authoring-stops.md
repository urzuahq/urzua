---
Status: Planned
Stable-Id: 01M2W199SQBR59QQ77A6FR11K5
Phase: '1'
Track: governance-process
Implements: SPEC-1
Depends-on: MILE-177
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

## A friction point found in this project's own use

`urzua new` writes a record and leaves it untracked, and `check` governs tracked files only
(`ADR-56`). Between the two, a freshly created record is invisible: a reference to it from an
already-tracked file is reported as dangling, at `error` severity, blocking, while the record sits on
disk. It happened twice in one session and was misdiagnosed both times as a stale build.

Staging what `urzua new` creates would close that window at the source. Recorded here rather than
decided, because it is a question about which path is faster -- this milestone's subject -- and not
about what is governed, which `ADR-56` settles. It should be configurable if it is built: a command
that touches the index is not what every workflow wants.

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

## Done means

Records in this repository are created through `urzua new` and not by hand: `urzua new` is measurably the faster path (a recorded comparison of time or steps against hand-authoring), the agent commands of `MILE-177` route through it, and over a stated window no record is merged whose header `urzua new` would have produced differently and `check` rejected; the window and the count are recorded here when it is met (`SPEC-1` Phase 1 exit criterion).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s criterion 4 had no milestone and was deleted from the spec alongside criterion 3. It is about preference under choice, not availability -- `urzua new` exists and records in this corpus were still hand-written, two of them rejected for header shapes the command would have produced correctly. | **substantive** |
> | 2026-11-04 | `Phase` 0 -> 1; `Status` unchanged. **Why:** Maintainer decision, 2026-11-04: `urzua new` being the faster path is a Phase 1 exit criterion, not part of the Phase 0 exit; `SPEC-1` already lists it under Phase 1. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-177` added. **Why:** Done means requires the agent commands of `MILE-177` to route through `urzua new` (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
