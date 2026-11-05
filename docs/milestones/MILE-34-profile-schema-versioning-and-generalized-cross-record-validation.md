---
Status: Planned
Stable-Id: 01M1Y5JYB0J2X7P0AVRJKWJAJV
Phase: '0'
Track: schema-governance
Implements: RFC-6
Depends-on: MILE-171
Blocked-on: —
---
# 34 — Profile schema versioning and generalized cross-record validation

## What

Version the profile schema itself (not just individual records), and generalize the ad hoc validation check/fix already does (reciprocity, dangling references) into a stated, pluggable model.

## Why

RFC-6 versions records; nothing versions the schema those records are validated against -- an explicitly open question in research with no answer yet.

## Done means

The profile schema carries a version that `check` reads and a mismatching version is reported with a migration path rather than a bare equality reject, and the ad hoc cross-record validation (reciprocity, dangling references) is stated as a declared model that the rule table (`MILE-171`) runs; a fixture on an older schema version is reported, and the versioning is exercised in urzua's own `.urzua/config.yaml`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): `schema_version` is a hard equality reject in `config.rs` with no migration path, and the pluggable cross-record validation model is what the rule-table work (`MILE-171`) builds on. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-171` added. **Why:** Done means requires the cross-record validation model to be run by the rule table `MILE-171` built (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
