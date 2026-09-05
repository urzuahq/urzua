# 0012 — The decision-before-implementation gate: refuse the merge, not the edit

> Status: Draft
> Date: 2026-08-12
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

Every mechanism this project has designed detects an inconsistency *after* it exists. Propose one
that prevents a class of them: **code carrying a back-pointer to a record that is not yet accepted
cannot merge.** The evidence it needs is the back-pointer the Embodiment model (RFC-0005) already
requires, so it costs almost nothing to add and enforces the ordering the whole product assumes —
decide, then build. Propose also the boundary it must not cross: it refuses the *merge*, never the
*edit*, because writing an implementation while a decision is pending is normal and is often what
reveals the decision was wrong.

## Motivation

Consider a codebase with this gate in place, over a stretch of work that implements five decisions:
the gate fires twice — both times correctly, both times on the author of the very decision being
implemented, both times catching an ordering that would otherwise have been held only in a human's
head.

That is the whole argument in miniature. The ordering "record the decision, get it accepted, then
build" is easy to state as policy and hard to enforce by memory alone. The other mechanisms in this repo do not help: Embodiment computes what was
built, `check` validates a record's shape, the drift gate compares stated against computed. None of
them asks *whether the decision was approved before the code arrived*.

Two further observations from the same body of evidence sharpen where the gate belongs:

- **It is nearly free.** The `Implements: ADR-NNNN` marker is already required to compute
  Embodiment. The gate is a status lookup on a pointer the tool has already parsed.
- **It is the only prevention in the set.** Instructions can drift below the checks meant to
  enforce them, and a boundary check can go for months without ever firing. Both are detection
  failures. A gate that refuses a merge does not depend on anyone reading output.

## Proposal

### 1. The rule

For every source file carrying a realization back-pointer (`Implements:` / `Verifies:`, or the
record-side `realized_by` pointer of RFC-0005 §2) to record R: if R's status is not
acceptance-bearing, the run fails.

Acceptance-bearing follows the schema's own declaration per profile (RFC-0001 §1a) rather than a
hardcoded list — an org whose ADR profile calls the state `Ratified` must not need a code change.

### 2. It refuses the merge, not the edit

The gate runs where a merge is decided — CI on a pull request — and **not** as a pre-write hook.
This is a deliberate limit, not an omission:

- Writing an implementation while a decision is pending is legitimate and frequently the activity
  that discovers the decision is wrong. A tool that blocks the edit blocks the learning.
- A pre-write hook on this rule would fire continuously during exactly the work that produces the
  decision, which is how a check gets disabled.

RFC-0007's harness may *surface* the pending state at write time. It may not refuse the write.

### 3. The message names the resolution, not just the violation

*"implements R, which is not Accepted (Status: Proposed) — a human Decider must accept it before
the implementation can merge."* The finding is not "you did something wrong"; it is "this needs a
decision from a person, and here is which person-shaped act is missing." That framing matters
because the correct response is almost never to edit the code.

### 4. What it deliberately does not do

It does not check that the *right* person accepted, that the acceptance is recent, or that the
implementation matches the decision. Those are RFC-0002's attestation problem, an expiry problem,
and the permanent content-scope ceiling in SPEC-0001 respectively. This gate answers one question —
was the decision approved before this code merges — and answers it mechanically.

### 5. Consequence for RFC-0008: Embodiment is not auto-repairable

Consider the drift gate failing on its own author within an hour of
existing: a decision was accepted claiming `Verified` when its evidence supported only
`Implemented`. Two repairs were available — write the missing test, or lower the stated claim — and
**the gate cannot distinguish them.**

RFC-0008 §1's eligibility test admits Embodiment: it is derivable, single-valued, non-destructive,
and reversible. But automatic repair would resolve every such case *by lowering the claim*, because
that is the mechanical answer, and the mechanical answer is the wrong one exactly when the evidence
is missing rather than the claim being wrong.

**So Embodiment must be excluded from Tier 1 auto-repair despite satisfying the eligibility test.**
This is the first field found where the test admits something it should not, and it means RFC-0008
needs a fifth clause or an explicit exclusion list. Recorded here rather than silently patched,
because it is a hole in that RFC's central abstraction.

## Open questions

- **Where does the gate sit for a decision that spans repositories?** A back-pointer in one
  repository to a record in another cannot resolve the status locally. This is the same single-repo
  assumption a cross-record coherence check can make elsewhere, and the roadmap's Phase 4
  cross-repo graph is where it gets answered.
- **What is the escape hatch, and does it need one?** An urgent fix implementing a decision nobody
  can accept in time is a real scenario. An explicit, logged override is the obvious answer and is
  also the thing that erodes the gate. Unresolved; the surveyed instance has no override and has not
  yet needed one.
- **Does it apply to test back-pointers?** `Verifies:` on a test for a pending decision is arguably
  fine — writing the test is part of evaluating the proposal. The surveyed instance gates both.
- **Interaction with the pending-supersession model.** A record that claims to supersede another
  while not yet accepted already has a "pending" convention; whether this gate should have the
  equivalent is undesigned.

## Non-goals

- **Blocking the edit.** See §2 — the point of the limit is that it protects the work that produces
  decisions.
- **Judging who accepted, or whether the acceptance is still valid.** RFC-0002 and attestation
  expiry.
- **Judging whether the implementation actually matches the decision.** SPEC-0001's permanent
  ceiling.
- **Replacing review.** The gate answers one mechanical question; a human still reads the diff.

## References

- RFC-0005 — the back-pointer model whose evidence this reuses.
- RFC-0007 — the harness that may surface the pending state but may not refuse the write.
- RFC-0008 §1 — the eligibility test §5 above finds a hole in.
- RFC-0001 §1a — declared profiles, so acceptance-bearing states are not hardcoded.
