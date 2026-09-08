---
Status: Planned
Stable-Id: 01M1Z2D54784252A0PQMKM8XV6
Phase: '2'
Track: write-path
Implements: ADR-14, ADR-20
Blocked-on: —
---
# 79 — A guarded edit command for agents, sharing fix apply's write path

## What

Design and build a guarded `urzua edit` command for human/agent-decided values (as opposed to
`fix --apply`'s tool-computed ones), reusing `fix --apply`'s existing write mechanics
(`apply_repair`'s find-field/replace/append-revision-log-row shape, `FixLock`, `resolve_identity`
-- ADR-20, already shipped) rather than growing a second, independently-drifting write path.
`fix --apply` writes under RFC-8/ADR-15's narrow eligibility test (the tool writing a value it
computed itself); `urzua edit` would write under a different, looser policy: an agent or human
supplies the new content, and the tool's job is guarding the mechanical parts of the edit --
refusing to let a full-record rewrite silently clobber `Stable-Id` or an already-resolved
`Author`/`Deciders`, auto-appending a correctly dated/classed revision-log row instead of a
hand-typed one, and re-running `check` immediately after to catch a broken edit before it's
committed. Not yet designed in detail: exact command interface, whether it edits a single field or
a whole section, how a caller supplies the revision-log Why.

## Why

Proposed live, right after ADR-38 was found to have a field silently revert to placeholder text via
a hand-edit. On closer review (while planning this milestone), that specific incident's root cause
was narrower and different: `field_state`'s placeholder-token list didn't recognize this project's
own retired placeholder convention, so `field.quality` -- which already runs on every `check` --
never had a chance to flag the regression. Fixed directly as BUG-5, which closes that incident on
its own; a write-path command wouldn't have prevented it either way, since a human/agent could
still type the wrong value through `edit` just as easily as through a raw file edit. `urzua edit`
remains worth building, but its real justification is forward-looking write-path infrastructure
for human/agent-decided values, not incident prevention -- and `fix --apply`'s write path, which
this milestone originally described as not yet existing, has since shipped in full (ADR-20).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Rescoped: `Implements` corrected from `ADR-15, ADR-19` to `ADR-14, ADR-20` (`fix --apply`'s write path has shipped, not deferred); `Why` corrected to attribute the ADR-38 incident to `field_state`'s placeholder-token gap (BUG-5, fixed separately), not a missing write primitive. **Why:** found live while planning this milestone in detail -- building `edit` as originally framed would not have prevented the incident it was justified by, and its `Implements` list cited a stale premise. Left `Status: Planned`; the command itself is still real, separate, unbuilt work. | **structural** |
