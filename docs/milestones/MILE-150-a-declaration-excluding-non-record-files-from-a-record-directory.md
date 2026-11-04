---
Stable-Id: 01M6JKNQ689T5T5AZDFZ3C71JQ
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/discovery.rs, code:rust/crates/urzua-core/src/config.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
---
# 150 — A declaration excluding non-record files from a record directory

## What

Add a configuration declaration that excludes named non-record files (for example an index `README.md`) from a record type's directory.

## Why

A `README.md` in a record directory is read as a record, so `header.required-fields`, `filename.identity-pattern` and `section.required` all fire on it: about a dozen findings on a real adopter corpus, none fixable by editing the file without making it a fake record. A type must be able to say which files in its directory are not its records. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A declared exclusion removes the file from every rule's population and from the type's file census; an undeclared stray file is still reported; the exclusion is documented in the spec that owns the type declaration; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Implemented in the discovery PR (not flipped to Done): per-type `exclude_files` (bare file names), dropped at record loading so the file is in no rule's population or census; documented in `SPEC-3` 0.18 and `SPEC-2` 0.31; exercised by `docs/adr/README.md` and the `adr` type's `exclude_files` in `.urzua/config.yaml`, with `filename.identity-pattern` enabled there. **Why:** the Done-when asks for a declaration, a still-reported stray file, the spec and an own-repository exercise. | **substantive** |
> | 2026-11-04 | Follow-up: an `exclude_files` entry matching no file is reported as a `type.dir-matches-nothing` warning (`SPEC-3` 0.24, `SPEC-2` 0.37). **Why:** a stale exclusion otherwise silently does nothing. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #192: the per-type `exclude_files` declaration removes a named non-record file from every rule population. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
