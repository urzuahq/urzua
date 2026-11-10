---
Stable-Id: 01M6JW46AMFZNJ0MNECN7Z4VJ1
Status: Done
Phase: '0'
Track: section-checks
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-core/src/rules/section.rs#a_declared_alias_satisfies_the_requirement_and_is_disclosed_observed_failing@sha256:79dcb9ab9f6b2357d9c70e4f0c73f2e9191ecbc39307cb9b41714daa80ae56b4, test:rust/crates/urzua-core/src/rules/section.rs#a_y_statement_in_the_preamble_is_judged_by_the_same_shape_check_observed_failing@sha256:202eefa1a850dd03ba3893a1527bb5e4731b70e033786f4d8f1071494f234a50, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-section-rules-run.yaml
---
# 165 — Section heading aliases, and reading a Y-statement outside any section heading

## What

Accept declared aliases for a section's heading, and read a Y-statement that sits outside any `##` section.

## Why

Real adopter records name the same section differently and sometimes carry the Y-statement in the preamble, so a single literal heading and a section-only reading produce false errors. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A declared alias satisfies the section requirement and is reported as the alias used. — evidence: test:rust/crates/urzua-core/src/rules/section.rs#a_declared_alias_satisfies_the_requirement_and_is_disclosed_observed_failing@sha256:79dcb9ab9f6b2357d9c70e4f0c73f2e9191ecbc39307cb9b41714daa80ae56b4
- A Y-statement found before the first `##` heading is judged by the same shape check. — evidence: test:rust/crates/urzua-core/src/rules/section.rs#a_y_statement_in_the_preamble_is_judged_by_the_same_shape_check_observed_failing@sha256:202eefa1a850dd03ba3893a1527bb5e4731b70e033786f4d8f1071494f234a50
- A fixture covers each. — evidence: test:rust/crates/urzua-core/src/rules/section.rs#a_declared_alias_satisfies_the_requirement_and_is_disclosed_observed_failing@sha256:79dcb9ab9f6b2357d9c70e4f0c73f2e9191ecbc39307cb9b41714daa80ae56b4, test:rust/crates/urzua-core/src/rules/section.rs#a_y_statement_in_the_preamble_is_judged_by_the_same_shape_check_observed_failing@sha256:202eefa1a850dd03ba3893a1527bb5e4731b70e033786f4d8f1071494f234a50
- The alias capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-section-rules-run.yaml
- The preamble reading is exercised by the fixture tests that read a record whose Y-statement sits before the first `##` heading and judge it. This repository's own ADRs predate the five-phrase Y-statement form (96 would fail the shape check), so the shape check and `preamble` are not declared here. — evidence: test:rust/crates/urzua-core/src/rules/section.rs#a_y_statement_in_the_preamble_is_judged_by_the_same_shape_check_observed_failing@sha256:202eefa1a850dd03ba3893a1527bb5e4731b70e033786f4d8f1071494f234a50, test:rust/crates/urzua-core/src/rules/section.rs#the_preamble_is_read_only_when_declared_and_only_when_the_marker_is_there@sha256:7c569c07393a2598fed7fb308d5f8dbc37c11c953a3b886afb4c143c1220b0b9

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: a `sections.required` entry takes `aliases` (other headings that satisfy it; both rules follow them; `check` reports one info notice per alias in use) and `preamble: true` (a `y-statement` with a `marker` is read from the text before the first `##` when its section is absent, and judged by the same shape check). Exercised in this repository: the `bug` type requires `What was wrong` with three aliases that 19 older bug records use. The preamble reading has fixture coverage only, because no record of this repository carries a Y-statement outside a section. Documented in `SPEC-2` and `SPEC-3`. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #222: section `aliases` and `preamble: true` Y-statement reading. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | `Status` Done -> InProgress. **Why:** the Done means requires the capability to be exercised by this repository's own config and records, and the 2026-11-04 row admits the `preamble: true` reading has fixture coverage only: no record here carries a Y-statement outside a section, and the config declares no `preamble`. `aliases` is exercised (the `bug` type). The code is built and tested (#222), so `status.delivery-agreement` computes Done from `Realized-by`; `WAIVER-0051` (expires 2026-12-07) holds the honest status until a record here exercises the reading or the maintainer amends the Done means. | **substantive** |
> | 2026-11-07 | Revision: `ADR-96` (Proposed) records the engine decision this record exposed. `judge_delivery` ignored Done means, so this milestone computed Done from one real locator; under `ADR-96` its Done means becomes items and the preamble item stays unresolved, so the engine computes `InProgress` itself. `WAIVER-0051` stays until `MILE-214` lands and is removed there. `Status` is unchanged. **Why:** maintainer decision of 2026-11-07 that status is computed from evidence per Done-means item. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `WAIVER-0051` removed; `Status` unchanged. **Why:** `statuses.delivery.items` is on for `milestone` (`MILE-215`), so this record's own Done means holds `InProgress`: its last item, the preamble reading exercised by this repository's own records, has no evidence. The waiver matched nothing. | **structural** |
> | 2026-11-10 | Last Done-means item rests on the fixture tests that exercise the preamble reading; this repository's ADRs predate the five-phrase form. **Why:** Maintainer decision, 2026-11-10. | **substantive** |
> | 2026-11-10 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (5 of 5 items resolved) computes to Verified` -- stated `InProgress` disagreed with the computed value `Done`. | **structural** |
