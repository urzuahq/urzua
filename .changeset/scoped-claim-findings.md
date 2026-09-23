---
default: patch
---

Fixes `BUG-121`: `check.rs`'s path-scope filter dropped a `claim.status-agreement` finding whenever
the claim file (under `claim_paths`) fell outside the requested `check <path>` scope, even though the
claim was genuinely checked — the same defect `BUG-86` documented but only worked around at this
repository's own Makefile caller, never fixed in the filter itself. A new declared list,
`rules::RULES_REPORTING_OUTSIDE_THE_CORPUS`, exempts `claim.status-agreement` findings from the
path-scope filter the same way a config-file finding already is exempted.

No adopter-facing behavior change for unscoped `check` (this repository's own invocation): verified
with the full test suite, clippy, `make ci`, and a real-corpus `check` run reporting the same 70
findings before and after. A new integration test exercises the previously-broken case directly: a
scoped `check docs/adr` over a false claim under `changes/` now correctly reports and blocks.

Three other candidates from this review pass were investigated and refuted as deliberate,
already-decided design: `supersession_reciprocity`/`embodiment_consistency`'s declaration-gating
(`ADR-53`'s "declared, not voted" principle, already the pattern throughout this release),
`header.rs`'s exact-case duplicate-key comparison (`ADR-57`, whose own doc comment anticipates and
answers exactly this scenario), and the TOML-to-YAML config break shipping no migration diagnostic
(`ADR-54`, decided with measured adoption evidence). A reported code-duplication finding (six rule
functions allegedly not sharing a slot-construction helper) was also refuted: five of the six already
use the shared `field_slots` helper: only `header_required_fields` doesn't, deliberately, for
record-scoped rather than field-scoped findings.
