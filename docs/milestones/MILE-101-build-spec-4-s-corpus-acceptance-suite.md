---
Status: InProgress
Stable-Id: 01M2VN4DCGV5PNZ3FWSZG4WQEJ
Phase: '1'
Track: schema-governance
Implements: SPEC-4
Depends-on: MILE-170
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

- Every fixture corpus under `urzua-cli/tests/acceptance/corpora/` has a `manifest.yaml` recording each irregularity it was built to exhibit. — evidence: test:rust/crates/urzua-cli/tests/acceptance.rs#check_manifest@sha256:a17a8a8c99e59fd9abcc194f8f0a211335ee294be7ffaade27cbffeba89b273d
- Every irregularity the manifests list has a case file pairing the corpus, a config, an invocation and an expected result.
- `tests/acceptance.rs` runs all of them in `cargo test --workspace`. — evidence: test:rust/crates/urzua-cli/tests/acceptance.rs#main@sha256:4e26f4ef2c8bb82b6a8d434b185670f7c15f0db5c3a8fa696918002355e159d9, code:rust/crates/urzua-cli/Cargo.toml
- A case whose expectation is removed fails the run.
- Bug-reproduction coverage is complete for the bugs `SPEC-4` names, and the harness half (mutation proof, coverage report) delivered by `MILE-170` is not repeated here.

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
> | 2026-11-05 | `Depends-on: MILE-170` added. **Why:** the What says the harness half moved to `MILE-170` and Done means relies on it (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: unspecified): recomputed `Status` from `the Done means items (2 of 5 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/acceptance.rs#main` (lines 820-1011) eed76a4aa5bf -> 4e26f4ef2c8b. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. acceptance.rs#main: main now routes the merge run (judging the coverage gate over every shard's proof file) before the case run, and the case run is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bug linkage landed (PR 0a): a case lists the bugs it reproduces in `bugs:`, and `acceptance_bug_gate.rs` holds every Fixed bug record to a tagged case, one of four shrink-only lane gap files, or a ledger of bugs no corpus run can show. Seeded with 7 bugs tagged on existing cases (`BUG-7`, `BUG-38`, `BUG-49`, `BUG-51`, `BUG-62`, `BUG-79`, `BUG-191`), 124 in the gap files (lane A 29, B 34, C 28, D 33) and 44 in the ledger. `Status` unchanged, and the Done means item on bug-reproduction coverage stays open until the lanes empty the gap files. **Why:** nothing linked a bug record to a case, so "every documented bug reproduces" was a hand count; the gate makes the remaining work a number that only falls. | **structural** |
