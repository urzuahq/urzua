---
Stable-Id: 01M20SH89CRA0KXRE2W8J3V6FM
Status: Draft
Date: 2026-09-08
Author: '@beauwilliams'
---
# 19 — Fix's future capability model: beyond Tier 1 Embodiment recomputation

## Summary

`fix` ([SPEC-8](../specs/SPEC-8-urzua-fix.md)) implements exactly one computation today: recompute
`Embodiment` from `Realized-by`'s cited evidence. Two further tiers were named in ADR-15 but never
built. This RFC opens one concrete, motivating case for a future tier — resolving a placeholder
`Author`/`Deciders` value from git history — without deciding whether or when to build it.

## Motivation

[MILE-78](../milestones/MILE-78-backfill-author-and-deciders-placeholder-text-with-the-real-github-handle.md)
found 92 instances of anonymized `Author`/`Deciders` placeholder text across this corpus and
backfilled them with the real handle, `@beauwilliams`, via a one-time corpus data fix — not through
`fix`, because ADR-15's eligibility test requires a field be "derivable from evidence already in the
record tree," and nothing in a record's own text tells you what its real author's handle is.

But that's narrower than what's actually available: **git history already knows who committed each
record's file**, the same class of evidence ADR-32 already treats as legitimate (git blame drives
`embodiment.consistency`'s drift detection today). `field_state::classify` can already recognize a
placeholder `Author` value (`PLACEHOLDER_TOKENS`, hardened by BUG-5 after missing this project's own
`(project lead)`/`(session author)` convention). The missing piece is connecting the two: detect a
placeholder, resolve the real value from the file's own commit history, report or write it back
under `fix`'s existing hard gates.

## Proposal

Sketch of a hypothetical future tier (**not Tier 2 or 3 as ADR-15 already named them** — those are
about relationship-completion and locator-repointing, a different shape of gap):

- **Detect**: for every record whose `Author`/`Deciders` classifies as `FieldState::Placeholder`,
  resolve the record's file's first commit (`git log --follow --format=%an --reverse -- <path> |
  head -1`, or the equivalent via `urzua_io`'s existing git-shelling patterns) and report it as a
  candidate value, the same shape as a Tier 1 finding. **Left genuinely unresolved by this sketch,
  not glossed over**: `Author` is single-valued but `Deciders` is "one or more" (SPEC-16) — one
  first-commit name can't populate both the same way, and a raw `git log` author name is a commit
  identity, not necessarily the same verified-or-fallback identity `resolve_identity()` already
  produces (`gh api user` > `git config user.name`, in that order) — the two could disagree for the
  same person. Any real design would need a separate mapping/cardinality rule per field, and to
  decide whether a bare commit-author name is even eligible evidence, or only a `gh`-verified one.
- **Apply**: write the resolved value back under the same hard gates `--apply` already enforces
  (resolved identity for the *operator* running the write, revision-log entry, byte-preserving
  everything else) — this writes a *different* record's historical field, not the operator's own
  attribution. A revision-log entry that just says "resolved from git history" would not actually
  satisfy ADR-14's own auditability bar (undoable *by inspection*) — it would need to name the exact
  commit and command used, the same specificity Tier 1's findings already report (`evidence: [...]`).
  Until that provenance shape is nailed down, this stays a **detect-only** sketch — apply is not
  proposed to ship alongside detect the way Tier 1's did.

## Open questions

- **Does the first-commit author reliably mean "the real author"?** Squashed history, rebased
  commits, or a commit authored by an agent under a shared bot identity could all make the git
  answer wrong in exactly the cases where a human hasn't reviewed it — the opposite of ADR-15's
  "single-valued, no inference" clause if the answer is actually ambiguous.
- **Is git commit history "evidence already in the record tree" at all, under ADR-15's own wording?**
  ADR-32 already treats git blame as legitimate evidence for locator-drift detection, but that's a
  different application (has this file's *cited path* changed) than this proposal's (who is this
  *record's* real author) — the precedent doesn't automatically transfer, and ADR-15's eligibility
  test was written before either use existed. Needs its own reasoning, not an inherited assumption.
- **Does this belong in `fix` at all, or is it `migrate`-shaped?** MILE-78 was a one-time backfill,
  not an ongoing repair loop the way Tier 1's Embodiment recomputation is (new drift can appear
  indefinitely; a placeholder-to-real-author gap, once backfilled once, mostly doesn't recur). ADR-33
  already declined to build a general, permanently-maintained capability "on the strength of" a
  single historical case — does the same reasoning apply here?
- **Should this wait for a second real case**, per this project's own standing practice (ADR-33,
  RFC-9's own framing) of not building speculative capability until a real case demands it a second
  time?

## Non-goals

- **Does not decide to build this.** MILE-78 already happened, manually, and was the right call at
  the time — this RFC exists to hold the "could a future tier do this mechanically" question
  somewhere durable, not to commit to answering it now.
- **Does not reopen ADR-15's existing Tier 1 eligibility test** — this is a candidate for a
  *different* tier, not a reinterpretation of the one that already shipped.

## References

- ADR-15 — the eligibility test any future tier must satisfy or explicitly can't.
- ADR-32 — git-blame as legitimate evidence, the precedent this proposal's core idea leans on.
- MILE-78 — the real case this RFC is a response to.
- BUG-5 — `field_state`'s placeholder-detection gap, fixed the same day MILE-78 landed.
- SPEC-8 — `fix`'s current, complete build; where a new tier would eventually need to land.
- RFC-20 — where `fix` sits relative to `check`/`doctor` in the overall command surface.
