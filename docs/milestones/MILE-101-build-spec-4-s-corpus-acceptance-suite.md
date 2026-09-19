---
Status: Planned
Stable-Id: 01M2VN4DCGV5PNZ3FWSZG4WQEJ
Phase: '0'
Track: schema-governance
Implements: SPEC-4
Blocked-on: —
---
# 101 — Build `SPEC-4`'s corpus acceptance suite

## What

`SPEC-4` specifies a corpus acceptance suite: fixture corpora, a `manifest.yaml` recording what
irregularity each fixture was built to exhibit, and one case file per acceptance case pairing a
corpus, a config, an invocation and an expected result.

**None of it exists.** Nothing reads `manifest.yaml`; the workspace has three test files
(`check_integration.rs`, `new_integration.rs`, `purity.rs`), none of them an acceptance suite. This is
why `SPEC-4` stayed `Draft` when `SPEC-1`, `2` and `3` were accepted (`BUG-26`).

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

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s second success criterion depends on an acceptance suite that does not exist, and no milestone tracked building it. The case for it is no longer the spec's: a review of one unreleased diff found eight defects, none covered by a test, and every one was corpus-shaped. | **substantive** |
