---
Stable-Id: 01M6JW46TQ0FA01XN38PPQS299
Status: Done
Phase: '0'
Track: embodiment-model
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_recent_last_audit_holds_back_drift_the_locator_dates_would_report@sha256:0a74ee212b0fd15e6978a776939d12421c36bdb7036395258c30228776faa000, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#an_old_last_audit_reports_drift_the_locator_dates_would_hold_back@sha256:9fb6963469b8fdbcdbefd3c244a5a2a853b7b737e3e6012e2a07b2e803e4dc59, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_record_without_the_audit_field_counts_from_the_files_last_change@sha256:1f0c40c1327afe284fa143f016c388135f1b55db9433aa26a476edaa084c76fa, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_malformed_last_audit_is_reported_and_falls_back_to_the_files_last_change@sha256:c1f6c10d611cf817dc1ab2255b24484aebb62ad97762a91c51cb3c4d9d04069d, code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-embodiment-rules-run.yaml
---
# 166 — `drift_grace_days` can count from a declared last-audit field

## What

Let the grace window of `drift_grace_days` (`MILE-129`) count from a declared last-audit date field on the record instead of the file's last change.

## Why

A record's file changes for reasons unrelated to its evidence, so the last change date mis-dates the audit; a real adopter corpus keeps the audit date in a field. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- When a last-audit field is declared and present, the window counts from it. — evidence: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_recent_last_audit_holds_back_drift_the_locator_dates_would_report@sha256:0a74ee212b0fd15e6978a776939d12421c36bdb7036395258c30228776faa000, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#an_old_last_audit_reports_drift_the_locator_dates_would_hold_back@sha256:9fb6963469b8fdbcdbefd3c244a5a2a853b7b737e3e6012e2a07b2e803e4dc59
- Otherwise the window counts from the file's last change as today. — evidence: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_record_without_the_audit_field_counts_from_the_files_last_change@sha256:1f0c40c1327afe284fa143f016c388135f1b55db9433aa26a476edaa084c76fa
- A malformed date is reported, not ignored. — evidence: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_malformed_last_audit_is_reported_and_falls_back_to_the_files_last_change@sha256:c1f6c10d611cf817dc1ab2255b24484aebb62ad97762a91c51cb3c4d9d04069d
- A fixture covers both sources. — evidence: test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_recent_last_audit_holds_back_drift_the_locator_dates_would_report@sha256:0a74ee212b0fd15e6978a776939d12421c36bdb7036395258c30228776faa000, test:rust/crates/urzua-cli/tests/evidence_pins_integration.rs#a_record_without_the_audit_field_counts_from_the_files_last_change@sha256:1f0c40c1327afe284fa143f016c388135f1b55db9433aa26a476edaa084c76fa
- The capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests. — evidence: code:.urzua/config.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-embodiment-rules-run.yaml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented: `embodiment.consistency` takes `drift_audit_field` (needs `drift_grace_days`, names a declared field); a record with a well-formed date there has its grace window counted from it, otherwise from the cited file's last change, and a malformed date is a warning. This repository declares `Last-audited` on specs and SPEC-3 carries one; `evidence_pins_integration.rs` covers both sources and the malformed case. `Status` stays `Planned`. **Why:** the Done-when contract. | **substantive** |
> | 2026-11-04 | `SPEC-3` no longer carries a `Last-audited` field: it had been set only to exercise this capability and no audit took place. The own-repo exercise of the audit-field source is covered by the integration tests; the field will be set by a real audit. **Why:** a date attesting to an audit that did not happen is a false attestation. | **structural** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: `drift_grace_days` counts from a declared last-audit field. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
