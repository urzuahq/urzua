---
Status: Planned
Stable-Id: 01M2VN4DCGV5PNZ3FWSZG4WQEJ
Phase: '1'
Track: schema-governance
Implements: SPEC-4
Blocked-on: —
---
# 101 — Build `SPEC-4`'s corpus acceptance suite

## What

`SPEC-4` specifies a corpus acceptance suite: fixture corpora, a `manifest.yaml` recording what
irregularity each fixture was built to exhibit, and one case file per acceptance case pairing a
corpus, a config, an invocation and an expected result.

**Part of it now exists** (see the structural revision row of 2026-11-04): `tests/acceptance.rs` is a
custom runner that reads case files and `manifest.yaml`, and four corpora and six cases are checked in.
What remains for this milestone is the other half of the criterion: every documented bug reproduces in
the suite. The harness itself moved to `MILE-170` (Phase 0).

## Why it is Phase 0

`SPEC-1`'s second success criterion:

> It reproduces **every documented bug** in the acceptance suite against real corpora. Not synthetic
> tests only -- fixtures engineered to exhibit the same variance a real corpus does.

Phase 0 has no work item for it, so the criterion has been unmet and untracked simultaneously.

## The evidence that it is needed

Two days of defects argue for it more sharply than the spec does. A review of one unreleased diff found
**eight defects, none covered by a test**, and `cargo test` passed on `main` with all eight present. Four of them:

- `audit` bypassing the rules table entirely (`BUG-42`)
- `init` adopting an `archive/` subdirectory and leaving three records ungoverned (`BUG-43`)
- a rule inverting into all-errors when an option was missing (`BUG-44`)
- a verb matched inside `prefixes`, raising blocking errors on correct prose (`BUG-45`)

Every one is a *corpus-shaped* defect -- a real directory layout, a real filename convention, a real
sentence -- and every one was invisible to unit tests written against the case the author had in mind.
`SPEC-4`'s own line is the diagnosis: *"The suite is red before the rule exists and green after. A case
that passes against a build with the rule removed is not a case."*

## `SPEC-4` needs a revision first

It specifies `manifest.toml` and `<case-id>.toml`, updated to `.yaml` under `ADR-52` but never built,
so the format has never met a real case. Expect the spec to change on contact -- and the change to be
`SPEC-4`'s revision log, not a silent divergence.

## Done means

Every fixture corpus under `urzua-cli/tests/acceptance/corpora/` has a `manifest.yaml` recording each irregularity it was built to exhibit, every irregularity the manifests list has a case file pairing the corpus, a config, an invocation and an expected result, and `tests/acceptance.rs` runs all of them in `cargo test --workspace`; a case whose expectation is removed fails the run, bug-reproduction coverage is complete for the bugs `SPEC-4` names, and the harness half (mutation proof, coverage report) delivered by `MILE-170` is not repeated here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s second success criterion depends on an acceptance suite that does not exist, and no milestone tracked building it. The case for it is no longer the spec's: a review of one unreleased diff found eight defects, none covered by a test, and every one was corpus-shaped. | **substantive** |
> | 2026-11-04 | `Phase` 0 -> 1; `Status` unchanged. **Why:** Maintainer decision, 2026-11-04: the full acceptance suite is a Phase 1 exit criterion, not part of the Phase 0 exit; `SPEC-1` already lists it under Phase 1. | **substantive** |
> | 2026-11-04 | Split: the acceptance harness (the runner, the `manifest.yaml` reader, the case format and `known_gap` markers, the private-corpus hook) is filed as `MILE-170` at Phase 0. This milestone stays at Phase 1 and now means "every documented bug reproduces in the suite against corpora", which is `SPEC-1`'s Phase 1 criterion. `Status` and `Phase` unchanged. **Why:** the refactors in the foundation set (`MILE-171` to `MILE-176`) need the harness as a safety net during Phase 0, while the bug-reproduction coverage is Phase 1 work. | **substantive** |
> | 2026-11-04 | Corrected the stale sentence "None of it exists" in What, which stopped being true when the first runner landed (`SPEC-4` 0.4, 2026-11-02): `urzua-cli/tests/acceptance.rs` and `tests/acceptance/corpora/{corpus-a,corpus-b,corpus-c,corpus-zero}` exist. The earlier rows are left as written. **Why:** a milestone that says nothing exists misleads the reader into rebuilding the runner; the record of what was believed at filing is kept in the log. | **structural** |
> | 2026-11-04 | `MILE-170` delivered the harness half (mutation proof, coverage report, property, "Nth record type" case, `corpus-d` to `corpus-f`); this milestone's bug-reproduction coverage is unchanged. `Status` and `Phase` unchanged. **Why:** keeps the split recorded on both sides. | **structural** |
> | 2026-11-04 | Evidence so far, no `Realized-by` yet. **Why:** the runner (`acceptance.rs`) and two corpora with manifests exist (merged in #146) and a property slice (`property.rs`, #97); the full suite `SPEC-4` specifies, every irregularity per manifest, is not. `Status` stays Planned. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
