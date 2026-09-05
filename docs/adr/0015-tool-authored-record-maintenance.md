# 0015 — Tool-authored record maintenance: derived fields are a cache the tool owns

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0008 (Accepted)

## Context

RFC-0008 proposed that Urzua write records, not only read them, for a narrow class of fields: those
whose correct value is mechanically derivable from evidence already in the tree. Warning-only leaves
a corpus decaying monotonically — every code movement produces a warning, warnings accumulate into an
unread stream, and the corpus's trustworthiness falls until nobody uses it. The reflex against
auto-writing comes from a real prior incident, but that incident rewrote authored prose in a corpus
with no revision log to make the rewrite recoverable — a different failure than overwriting a stale
copy of a computation ADR-0014's revision log now makes reversible.

## Decision

In the context of a validator that can compute a field's correct value but, until now, could only
warn when the stated value disagreed, facing the risk of collapsing "may rewrite a derived field" and
"may rewrite prose" into one blanket prohibition, we decided that a field is **tool-writable** if and
only if it is derivable (no inference), single-valued, non-destructive on write (the value it
replaces is a stale copy of the same computation, not human-composed content), and reversible (via
ADR-0014's revision log). Anything failing any clause stays permanently hand-authored — no confidence
score makes prose eligible.

Three repair tiers follow, gated independently and shipped in increasing order of nerve required:
recompute a derived field in place (Tier 1, on by default), complete a half-stated relationship
additively (Tier 2, opt-in), and repoint a stale locator only when the move is unambiguously
recoverable (Tier 3, opt-in, shipped last). Delivery reuses RFC-0002's detect/apply split exactly —
`urzua fix` in detect mode is read-only and safe in CI; apply requires an identity and re-verifies
each finding still holds at write time.

## Reversibility

Every applied write appends a `structural` revision-log entry (ADR-0014) naming the computation and
the evidence it read, so a tool-authored write is always undoable by inspection of that entry. The
eligibility test itself is conservative by construction — Embodiment is explicitly excluded despite
satisfying every clause, because auto-repair would resolve a stated-exceeds-computed mismatch by
lowering the claim, which is the mechanical answer and the wrong one whenever the evidence is merely
missing rather than the claim being wrong (RFC-0012 §5 carries this argument).

## Consequences

- `urzua fix` is a new command: detect mode (default, read-only, CI-safe) emits one entry per
  repairable fact with `record`, `field`, `currentValue`, `computedValue`, `tier`, `evidence`, and
  `suggested_action`; apply mode requires `--ids` and `--by <identity>`, or `--force --by <identity>`
  for a batch bypass that is never the default.
- `--tier` defaults to 1; Tiers 2 and 3 are opt-in per invocation, consistent with RFC-0008's
  increasing-nerve ordering.
- A derivation is only trustworthy as its own test: any field this ADR permits a tool to author needs
  a test that fails against a plausible *wrong* implementation, not merely one that passes against
  the correct one, and the query or computation itself must be named in the record it produces —
  never just "derived from history."
- Apply is single-process, holds a corpus-level lock, and re-verifies each ID still shows as
  repairable at write time — a stale ID under a batch apply must not write a value computed against a
  tree that no longer exists.
- Deleting a rotted derived literal from prose remains explicitly out of scope: the tool reports it,
  a human deletes it. The clearest available illustration of where the authored/derived line sits is
  that the most obviously-wrong value in a corpus can still be the one thing this ADR refuses to
  touch.
- This ADR directly unblocks the `migrate` bulk-rewrite tier for any newly-required field whose value
  passes the eligibility test above — but not for fields that don't (most newly-required fields, e.g.
  `Reviewers`, require human authorship and stay in the waiver-assist path, never this one).
- Several open questions are explicitly deferred rather than resolved here: whether Tier 1 should run
  automatically in the pre-commit hook, the repair-vs-supersede boundary, cross-branch handling for
  Tier 3, and whether the single-valued clause holds for display numbers under concurrent merges
  (SPEC-0001's hardest open correctness problem).

## References

- RFC-0008 — the proposal this decides.
- RFC-0002 — the detect/apply split and identity resolution this ADR reuses rather than redesigns.
- ADR-0014 — the revision log this ADR's reversibility clause depends on.
- RFC-0012 §5 — the argument for why Embodiment is excluded despite satisfying the eligibility test.
