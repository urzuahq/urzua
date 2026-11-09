---
Stable-Id: 01M6R3NK4TA9Q1295W1E10MVXK
Status: Accepted
Embodiment: Implemented
Realized-by: code:rust/crates/urzua-core/src/header.rs@sha256:138444217009c90bb532bed4f31605269e6a53ccde85651e142fc1da5ea1e654
Date: 2026-09-04
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: RFC-10
---
# 8 — The record header is a closed structure, validated as a whole

## Context

RFC-10 proposed that a record's header be treated as a closed structure — a bounded region
containing a known key set, each key appearing exactly once, nothing else — rather than validated
field by field. Per-field checks can't ask "is this the header, all of it, and only it," no matter
how many fields are checked.

Phase 3 built exactly this: `urzua-core::header` locates the header by anchoring to the metadata
shape (never a positional slice), handles all three shapes actually present in this corpus
(blockquote, bold-labelled, pipe-delimited), reports duplicate keys as errors rather than
first-wins, and reports "no header found" as a finding rather than an empty result. Running it
against this repository's own `docs/` confirmed the exact defect the RFC predicted: `SPEC-1`'s
header shape doesn't match `SPEC-2` through `SPEC-5`'s.

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
  this project's own corpus (`SPEC-1` vs. `SPEC-2`–`0005`) — fixing it is now a tracked,
  visible finding rather than an invisible inconsistency.
- Unknown-key enforcement (warn vs. error by default) remains an open question, deferred to Phase 6
  rather than decided here.

## References

- RFC-10 — the proposal this decides.
- `rust/crates/urzua-core/src/header.rs` — the implementation, including the planted-violation
  tests proving the three-shape handling and duplicate-key detection actually work.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Revision log added, 2026-11-05. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/header.rs@sha256:6bce44f1b7cf Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/header.rs` 6bce44f1b7cf -> 138444217009. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: header.rs: split parse_yaml_frontmatter into header_without_region and frontmatter_fields; the opening-line, closing-line and YAML-error checks stay in parse_yaml_frontmatter in the same order with the same parse_error text. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline and reverify JSON compared before and after, with and without --as-of). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
