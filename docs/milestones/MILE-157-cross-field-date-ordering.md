---
Stable-Id: 01M6JKNVFM208DS7MJ5A0TAMEE
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/date_order_integration.rs#an_adr_accepted_before_it_was_written_is_reported@sha256:e9463c2ece954d00ff50e92789741a2d11538c4253fd8747fd9c8363800671a9, test:rust/crates/urzua-cli/tests/date_order_integration.rs#equal_later_and_absent_acceptance_dates_pass@sha256:c6d1e687db2680dad7198137e8ca3f657873b01fc3b25037bb26e1d1c376ee9d, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-date-rules-run.yaml
---
# 157 — Cross-field date ordering

## What

Add a rule that checks one date field is not earlier than another in the same record (for example an acceptance date earlier than the record's own date).

## Why

A real adopter checks that a record is not accepted before it was written. urzua has no cross-field date check, so a verifiably impossible chronology passes; the dates are already declared fields, so the check needs no new evidence. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A configured pair of date fields is compared and an out-of-order pair is reported. — evidence: test:rust/crates/urzua-cli/tests/date_order_integration.rs#an_adr_accepted_before_it_was_written_is_reported@sha256:e9463c2ece954d00ff50e92789741a2d11538c4253fd8747fd9c8363800671a9
- Equal dates pass. — evidence: test:rust/crates/urzua-cli/tests/date_order_integration.rs#equal_later_and_absent_acceptance_dates_pass@sha256:c6d1e687db2680dad7198137e8ca3f657873b01fc3b25037bb26e1d1c376ee9d
- An absent or unparseable date is left to the existing field-quality rules. — evidence: test:rust/crates/urzua-cli/tests/date_order_integration.rs#equal_later_and_absent_acceptance_dates_pass@sha256:c6d1e687db2680dad7198137e8ca3f657873b01fc3b25037bb26e1d1c376ee9d
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-date-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented as the opt-in `date.field-order` rule reading `date_order: ["Date <= Accepted-on"]`, reusing `date.field-valid`'s date check and field states so the two never duplicate. This repository declares `Accepted-on` as an optional field of `adr` and `rfc`, validates it and orders it after `Date`; no existing record carries an acceptance date, so the corpus run is disclosed as eligible-and-unexamined and `date_order_integration.rs` supplies the planted violations. `Status` stays `Planned`. **Why:** the Done-when contract; backfilling acceptance dates onto Accepted records is a maintainer decision. | **substantive** |
> | 2026-11-04 | Follow-up: `date_order` takes structured entries `{earlier: Date, later: Accepted-on}` instead of the `Earlier <= Later` string (`SPEC-3` 0.30). **Why:** `ADR-64`, structured in, structured out; unreleased, so no alias. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #202: opt-in rule `date.field-order`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
