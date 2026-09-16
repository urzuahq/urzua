---
Stable-Id: 01M2MPDDSYA64NTGXWAZ9HRRGJ
Status: Planned
Phase: '1'
Track: schema-governance
Implements: —
Blocked-on: —
---
# 94 — A record's prose can assert what the code contradicts, and no rule reads it

## What

All seventeen `check` rules read header fields. Only two touch body text -- `filename.title-consistency`
reads the H1 line, `revision-log.change-class-required` reads the log table -- and both structurally.
**No rule reads a record's prose as a claim.** A record's body can assert anything about the code and
nothing contradicts it.

This milestone owns closing that, and the shape is already proven: **move load-bearing claims out of
prose and into checked fields.** Not "make prose checkable," which is not a tractable problem.

## Why

Six stale claims were found in one session on 2026-09-16, five of them structurally uncatchable:

| Stale claim | Caught? |
|---|---|
| `ADR-45`'s `Embodiment: Verified`, false from its first commit | **yes** -- a header field; warning among 204, so nothing acted (`MILE-80`) |
| `ADR-46` and `ADR-23`: "`init` and `migrate ids` remain plain-text prose end to end today" -- `BUG-20` fixed both | no |
| `ADR-45`: `verify-ci` works because the head "gated the merge" -- `main` has no branch protection | no |
| `SPEC-20`: three claims disproved by the `v0.2.1` run | no |
| `MILE-82`, `MILE-39`, `MILE-40`: `Planned`/`Blocked` for shipped work | no |

One caught and ignored; five that no mechanism could reach.

**The remedy is evidenced, not hypothetical.** `MILE-2` and `MILE-3` both sat with blockers that had
silently resolved. Their revision logs record what happened next:

> Cleared `Blocked-on` (was `BUG-3`, `Status: Fixed` for a while, caught by `blocked-on.stale`
> (`MILE-83`) **immediately after `Blocked-on` moved from prose into a checked header field**)

The rule found both the moment the claim stopped being prose. That is the pattern to repeat.

**And this project already predicted the failure.** `RFC-7` §1:

> an instruction living in a system prompt, an `AGENTS.md`-style file, a Cursor rule, or a
> `CLAUDE.md` convention is a *request*, not a *guarantee* ... should be treated as **unenforced**
> until there's a mechanism that fires independent of whether the agent remembered to follow it.

`AGENTS.md` instructs: when a change invalidates a claim in prose, grep the corpus for that claim's
key phrase. Every one of the five was written by an agent that had read that instruction. The gap is
not diligence, which is exactly what `RFC-7` ruled.

## Scope

Identify which prose claims are load-bearing enough to become fields, and for each decide the field
and the rule that checks it. Candidates surfaced by the above, not a settled list:

- **A record asserting the state of a command or file** (`ADR-46`/`ADR-23` on `init`/`migrate ids`).
  Possibly an extension of `Realized-by` with an assertion, or a narrower "this record describes the
  current behaviour of X" pointer.
- **A milestone's `Status` versus whether its work shipped.** No rule can know this in general. The
  tractable version is a reciprocal: a milestone whose `Realized-by` locators all exist and whose
  cited records are terminal is a candidate for review, in the shape of
  `embodiment.locator-promotion-candidate` -- surfaced, never auto-closed.
- **Amendment claims about external systems** (branch protection, trigger behaviour). Likely *not*
  fieldable; the honest answer may be that such claims must cite an observed run, which `doctor`
  could verify.

## Relation to existing work

- `MILE-80` -- would make the one caught instance blocking. Necessary, not sufficient: it does
  nothing for the five that were never reported.
- `MILE-86` -- prose citing a record with no structured pointer. The same family, narrower: it
  detects a *missing* pointer, not a *false* statement.
- `MILE-83` -- `Done`, and the proof this approach works.
- `RFC-28` -- the mirror direction: source files claiming to implement a record.

## References

- RFC-7 -- an instruction is not a mechanism; the prediction this milestone is the evidence for.
- MILE-83, MILE-2, MILE-3 -- the proven instance of moving a claim from prose into a field.
- MILE-80, MILE-86, RFC-28 -- adjacent, each covering a different slice.
- ADR-47, BUG-33, BUG-34 -- the session that surfaced the six instances.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial record, `Status: Planned`. **Why:** five stale claims merged into `main` in one session with every check green, because no rule reads prose. Filed as a named gap rather than five corrections, since correcting them individually leaves the mechanism that permitted them untouched -- and this project's own `RFC-7` already ruled that the instruction meant to prevent it is unenforced by construction. | **structural** |
