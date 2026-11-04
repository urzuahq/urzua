---
Stable-Id: 01M6JKNM82HMHR4D1TCD2B4WMG
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-core/src/rules/back_pointer.rs, test:rust/crates/urzua-cli/tests/test_tier_integration.rs
---
# 146 — A test-tier back-pointer so a record stating Verified is computed as Verified

## What

Read a back-pointer from a test file (for example `Verifies: ADR-N`), or otherwise let a declared file class count as test evidence, so the computed Embodiment can reach the verified tier from tests that cite a record.

## Why

A real adopter corpus has a few hundred records stating `Verified`. urzua computes `Implemented` for them because no test-tier pointer is read, so the only way to agree is a very long hand-maintained list of `test:` locators per record, some lines thousands of characters long. That is the opposite of urzua's principle that evidence is read from the artifact rather than restated beside it, and it rots the moment a test moves. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A test file carrying a back-pointer to a record is read as test evidence; a record whose only extra evidence is such tests computes `Verified`; the file class is declared in configuration; a fixture covers both a test-pointer and a non-test pointer to the same record; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Test-tier back-pointer (`SPEC-2` `0.30`, `SPEC-3` `0.16`): `Verifies:` in a file under the declared `test_paths` computes `Verified`; exercised by `purity.rs` citing `ADR-5` in this repository, plus a fixture citing one record at both tiers. **Why:** the Done-when asks for it. Status unchanged; the maintainer closes. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #198: `Verifies:` in a file under the declared `test_paths` computes `Verified`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
