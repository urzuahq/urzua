---
Status: Planned
Stable-Id: 01M1Y5JYB0J2X7P0AVRJKWJAJV
Phase: '0'
Track: schema-governance
Implements: RFC-6
Blocked-on: —
---
# 34 — Profile schema versioning and generalized cross-record validation

## What

Version the profile schema itself (not just individual records), and generalize the ad hoc validation check/fix already does (reciprocity, dangling references) into a stated, pluggable model.

## Why

RFC-6 versions records; nothing versions the schema those records are validated against -- an explicitly open question in research with no answer yet.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): `schema_version` is a hard equality reject in `config.rs` with no migration path, and the pluggable cross-record validation model is what the rule-table work (`MILE-171`) builds on. | **substantive** |
