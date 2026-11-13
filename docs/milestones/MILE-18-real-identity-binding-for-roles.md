---
Status: Planned
Stable-Id: 01M1Y5JNM6KSG8K1XBT7YMV9FB
Phase: '1'
Track: accountability-identity
Implements: —
Blocked-on: —
---
# 18 — Real identity binding for roles

## What

Resolve `Reviewers`/`Deciders` to a real account (GitHub/SSO) instead of free-text names.
`resolve_identity()` (`gh api user` -> `git config user.name` -> explicit `--by`, RFC-2 §2's
tiering) already does this for `Author`, wired into `urzua new` and `fix --apply` -- this milestone
is now scoped to what's actually still missing: `Reviewers`/`Deciders` stay free text always, never
tool-resolved, and nothing protects a resolved `Author` from being silently overwritten by a later
hand-edit (the exact regression ADR-38 had, fixed live this session -- MILE-79 tracks the general
guardrail for that).

## Why

A free-text name silently rots when the person leaves. RFC-2/ADR-31 already closed this for `Author`
on tool-authored writes; `Reviewers`/`Deciders` never got the same treatment, and a resolved value
has no protection against a later hand-edit clobbering it back to free text.

## Done means

- `Reviewers` and `Deciders` resolve to a real account through `resolve_identity()` (`gh api user`, then `git config user.name`, then explicit `--by`) wherever the tool writes them.
- A free-text value written by hand in `Reviewers` or `Deciders` is reported by `check`.
- A resolved `Author` hand-edited back to free text is reported rather than silently kept.
- Each is covered by a fixture.
- Each is exercised in urzua's own config and corpus.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Rescoped: `Author` resolution was already shipped (`resolve_identity()`, wired into `new`/`fix --apply`) and this milestone's original text didn't reflect it. **Why:** found live during backlog triage -- the milestone described a problem partially already solved, which would have led to redundant work if picked up as originally written. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
