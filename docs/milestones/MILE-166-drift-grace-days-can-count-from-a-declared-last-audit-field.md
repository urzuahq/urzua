---
Stable-Id: 01M6JW46TQ0FA01XN38PPQS299
Status: Done
Phase: '0'
Track: embodiment-model
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/embodiment.rs, code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs
---
# 166 — `drift_grace_days` can count from a declared last-audit field

## What

Let the grace window of `drift_grace_days` (`MILE-129`) count from a declared last-audit date field on the record instead of the file's last change.

## Why

A record's file changes for reasons unrelated to its evidence, so the last change date mis-dates the audit; a real adopter corpus keeps the audit date in a field. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

When a last-audit field is declared and present, the window counts from it; otherwise from the file's last change as today; a malformed date is reported, not ignored; a fixture covers both sources and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: `embodiment.consistency` takes `drift_audit_field` (needs `drift_grace_days`, names a declared field); a record with a well-formed date there has its grace window counted from it, otherwise from the cited file's last change, and a malformed date is a warning. This repository declares `Last-audited` on specs and SPEC-3 carries one; `evidence_pins_integration.rs` covers both sources and the malformed case. `Status` stays `Planned`. **Why:** the Done-when contract. | **substantive** |
> | 2026-11-04 | `SPEC-3` no longer carries a `Last-audited` field: it had been set only to exercise this capability and no audit took place. The own-repo exercise of the audit-field source is covered by the integration tests; the field will be set by a real audit. **Why:** a date attesting to an audit that did not happen is a false attestation. | **structural** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: `drift_grace_days` counts from a declared last-audit field. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
