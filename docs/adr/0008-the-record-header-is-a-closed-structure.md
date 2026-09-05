# 0008 — The record header is a closed structure, validated as a whole

> Status: Accepted
> Embodiment: Implemented
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-0010 (Accepted)

## Context

RFC-0010 proposed that a record's header be treated as a closed structure — a bounded region
containing a known key set, each key appearing exactly once, nothing else — rather than validated
field by field. Per-field checks can't ask "is this the header, all of it, and only it," no matter
how many fields are checked.

Phase 3 built exactly this: `urzua-core::header` locates the header by anchoring to the metadata
shape (never a positional slice), handles all three shapes actually present in this corpus
(blockquote, bold-labelled, pipe-delimited), reports duplicate keys as errors rather than
first-wins, and reports "no header found" as a finding rather than an empty result. Running it
against this repository's own `docs/` confirmed the exact defect the RFC predicted: `SPEC-0001`'s
header shape doesn't match `SPEC-0002` through `SPEC-0005`'s.

## Decision

In the context of per-field header checks that cannot express "and nothing else," facing three
real, competing header shapes in this project's own corpus, we decided to **treat the header as a
closed structure with a single shared parser**: the region is the maximal contiguous run of
metadata-shaped lines anchored at the first such line; unknown keys are reported (not yet enforced
as an error by default — that phasing is Phase 6 scope); a repeated key is always an error; a field
lookup outside the header region never counts as satisfying it.

Accepting the config-declared-required-fields shape for "what must be present," per record type,
rather than the RFC's more general profile-declared closure — the narrower version is what Phase 3
actually built and validated, and it can grow into the fuller model without breaking the config
shape.

## Reversibility

The parser is internal; nothing external depends on its exact heuristics yet. Cheap to extend
(e.g. tightening unknown-key handling from reported-only to error-by-default) once there's a real
adopter's config to test that transition against.

## Consequences

- Every future rule that reads a header field must go through this one parser — no check may run
  its own regex over raw document text, or the closure guarantee is only as strong as its least
  careful caller.
- The three-header-shape defect this parser exists to catch is real and currently unresolved in
  this project's own corpus (`SPEC-0001` vs. `SPEC-0002`–`0005`) — fixing it is now a tracked,
  visible finding rather than an invisible inconsistency.
- Unknown-key enforcement (warn vs. error by default) remains an open question, deferred to Phase 6
  rather than decided here.

## References

- RFC-0010 — the proposal this decides.
- `rust/crates/urzua-core/src/header.rs` — the implementation, including the planted-violation
  tests proving the three-shape handling and duplicate-key detection actually work.
