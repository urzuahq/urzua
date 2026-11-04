---
Stable-Id: 01M6HZ0Q0A3NTHSZ5WV1A9GJHA
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
---
# 143 — Check that cited paths and issue or PR numbers exist

## What

A warning-severity check over the citations in a record's references: a backtick-quoted repository path must exist inside the repository, and an issue or pull request reference (`#N` or a `/pull/N` link) must resolve to a real issue or pull request. It needs network access and a token, so it is opt-in and reports `not-run` when either is missing instead of passing.

## Why

The adopter checks both in the References section of every decision, proposal and spec record, as warnings. `embodiment.locator-exists` covers only `Realized-by` locators, and `MILE-37` and `MILE-108` are adjacent (README links, source comments) rather than the same check. `check` is otherwise offline and deterministic, so the network dependency has to be explicit and separable from the blocking run.

## Done means

A cited path that does not exist and a cited issue number that does not exist each produce a warning; a run without a token reports `not-run` with the reason; the rule never makes the default offline `check` non-deterministic; batching and rate-limit handling are covered by a test against a recorded fixture.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter warns on cited paths and issue or PR numbers that do not exist, and urzua checks neither. | **substantive** |
> | 2026-11-04 | Noted as a capability a real adopter corpus needs (parity gap, same class as `MILE-146` to `MILE-159`); no change to `Status`, `Phase` or scope. **Why:** a trial run of urzua over a real adopter corpus showed it checks cited paths and issue or PR numbers that urzua does not. | **substantive** |
> | 2026-11-04 | Built the cited-path half: opt-in `citation.path-exists` (`reference_sections` names the headings read; a backtick-quoted path must be tracked and on disk, or a directory with tracked files below it), offline and deterministic, documented in `SPEC-2` and `SPEC-3`. Not built: the issue and pull request half, which needs a decision on how existence is established offline-deterministically in CI (a committed snapshot, or an opt-in network run reporting `not-run`), and the recorded-fixture test for batching and rate limits that follows from it. Not yet enabled in this repository's own `.urzua/config.yaml`: it would report 32 warnings across 150 cited paths, mostly historical records naming paths that no longer exist, which `fail_on: warning` would turn into a failing build. `Status` is untouched. **Why:** the issue and PR design and the own-repo rollout are decisions the record does not settle. | **substantive** |
> | 2026-11-04 | Own-repo rollout of the cited-path half: `citation.path-exists` is enabled in this repository's `.urzua/config.yaml` with `reference_sections: ["References"]` at `warn`, under `fail_on: warning`. Stale paths were corrected in place (current path, or a plain statement that the file is gone); revision-log rows that name a path as it was at the time are history and are covered by narrow, expiring waivers instead of being rewritten. The issue and pull request number half is unchanged. `Status` is untouched. **Why:** the Done-when requires the rule to run clean in urzua's own repository. | **substantive** |
> | 2026-11-04 | Added the optional `skip_tables` option to `citation.path-exists`: the revision-log label as a marker sets aside that line and the pipe table after it, so append-only history that names a path as it was is not read. Empty by default, so behaviour is unchanged unless declared; this repository declares the revision-log label, and the 20 waivers that covered history rows were deleted. `Status` is untouched. **Why:** the revision log sits under `## References`, and its rows are correct as history; a waiver per record was the wrong tool for a whole class of text. | **substantive** |
