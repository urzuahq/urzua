---
Stable-Id: 01M6JW46AMFZNJ0MNECN7Z4VJ1
Status: InProgress
Phase: '0'
Track: section-checks
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/section.rs, test:rust/crates/urzua-cli/tests/section_rules_integration.rs
---
# 165 — Section heading aliases, and reading a Y-statement outside any section heading

## What

Accept declared aliases for a section's heading, and read a Y-statement that sits outside any `##` section.

## Why

Real adopter records name the same section differently and sometimes carry the Y-statement in the preamble, so a single literal heading and a section-only reading produce false errors. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared alias satisfies the section requirement and is reported as the alias used; a Y-statement found before the first `##` heading is judged by the same shape check; a fixture covers each and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: a `sections.required` entry takes `aliases` (other headings that satisfy it; both rules follow them; `check` reports one info notice per alias in use) and `preamble: true` (a `y-statement` with a `marker` is read from the text before the first `##` when its section is absent, and judged by the same shape check). Exercised in this repository: the `bug` type requires `What was wrong` with three aliases that 19 older bug records use. The preamble reading has fixture coverage only, because no record of this repository carries a Y-statement outside a section. Documented in `SPEC-2` and `SPEC-3`. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #222: section `aliases` and `preamble: true` Y-statement reading. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | `Status` Done -> InProgress. **Why:** the Done means requires the capability to be exercised by this repository's own config and records, and the 2026-11-04 row admits the `preamble: true` reading has fixture coverage only: no record here carries a Y-statement outside a section, and the config declares no `preamble`. `aliases` is exercised (the `bug` type). The code is built and tested (#222), so `status.delivery-agreement` computes Done from `Realized-by`; `WAIVER-0051` (expires 2026-12-07) holds the honest status until a record here exercises the reading or the maintainer amends the Done means. | **substantive** |
> | 2026-11-07 | Revision: `ADR-96` (Proposed) records the engine decision this record exposed. `judge_delivery` ignored Done means, so this milestone computed Done from one real locator; under `ADR-96` its Done means becomes items and the preamble item stays unresolved, so the engine computes `InProgress` itself. `WAIVER-0051` stays until `MILE-214` lands and is removed there. `Status` is unchanged. **Why:** maintainer decision of 2026-11-07 that status is computed from evidence per Done-means item. | **substantive** |
