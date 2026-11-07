---
Status: Done
Stable-Id: 01M1YNNAYPRGKFY23FMFWCQ907
Phase: '0'
Track: accountability-identity
Implements: ADR-11
Blocked-on: —
Realized-by: code:.urzua/config.yaml#record_types::waiver@sha256:40c9440519e08654700200a319a29023aa88badc65e3116a729abba86b4130c3, code:.urzua/templates/waiver.md, test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:037056b7b5b15eb088207e5d6dad10da14802a378f408054e5a31dbd22c3fac0, code:rust/crates/urzua-core/src/waiver.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#a_waiver_record_suppresses_blocking_but_the_finding_stays_listed@sha256:e3a4471c59e291ed5acea13743340d795e1a8864e0f72e15091702f235fc017b, test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b, spec:docs/specs/SPEC-10-the-waiver-record-type.md, code:docs/waiver/WAIVER-0035-bug-10-needs-no-test-its-fix-is-to-records-and-a-template.md
---
# 59 — Add the waiver record type

## What

The `waiver` record type: a reviewed exception to a rule is its own record (`Rule`, `Scope`, `Reason`, optional `Expires`), never a config-level ignore list. A waived finding stays listed, just excluded from the blocking exit code.

## Why

An ignore list is unreviewable and grows forever; a waiver record is a normal record with an author, a reason, and an expiry -- reviewable in the same diff as everything else.

## Done means

- `waiver` is a configured record type (`Rule`, `Scope`, `Reason` required, optional `Expires`) with a template. — evidence: code:.urzua/config.yaml#record_types::waiver@sha256:40c9440519e08654700200a319a29023aa88badc65e3116a729abba86b4130c3, code:.urzua/templates/waiver.md
- `urzua new waiver` works end to end. — evidence: code:.urzua/templates/waiver.md, test:rust/crates/urzua-cli/tests/new_integration.rs#new_over_a_padded_identity_pattern_passes_check_with_no_edit_observed_failing@sha256:037056b7b5b15eb088207e5d6dad10da14802a378f408054e5a31dbd22c3fac0
- A finding matched by a waiver stays listed but is excluded from the blocking exit code. — evidence: code:rust/crates/urzua-core/src/waiver.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#a_waiver_record_suppresses_blocking_but_the_finding_stays_listed@sha256:e3a4471c59e291ed5acea13743340d795e1a8864e0f72e15091702f235fc017b
- An expired or unmatched waiver is reported. — evidence: test:rust/crates/urzua-cli/tests/waiver_stale_integration.rs#a_whole_run_reports_the_stale_and_the_expired_waiver_but_not_the_live_one@sha256:57d141fb74f1a71d166722df1ebaa3183a462af600b6d3c970df438eca635d3b
- `docs/waiver/` holds this repository's own reviewed exceptions (`SPEC-10`). — evidence: spec:docs/specs/SPEC-10-the-waiver-record-type.md, code:docs/waiver/WAIVER-0035-bug-10-needs-no-test-its-fix-is-to-records-and-a-template.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-04 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-09-07 | Reopened: `Done` was inaccurate. `waiver.rs`'s parsing/matching logic existed and was tested, but the record type was never actually registered in `.urzua/config.toml`'s `record_types`, so `urzua new waiver` failed outright and no corpus author could ever create one -- ADR-11's own stated consequence ("must be added to `.urzua/config.toml`'s `record_types`... no special-casing") was never carried out. **Why:** found live, reasoning about why ADR-1's stale `Embodiment` couldn't be waived -- there was no way to create a waiver record at all. | **substantive** |
> | 2026-09-07 | Fixed: added `[record_types.waiver]` (`dir = "docs/waiver"`, `required_fields = ["Rule", "Scope", "Reason"]`) and `.urzua/templates/waiver.md`. Verified `urzua new waiver` end-to-end against an isolated scratch corpus (not this one, since there is no real waiver to add yet). `docs/waiver/` itself is left uncreated, same as every other type's directory before its first real record -- no placeholder file needed. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
