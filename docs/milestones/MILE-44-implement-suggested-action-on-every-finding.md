---
Status: Planned
Stable-Id: 01M1YN83N5WHAE63XN53HTDGHR
Phase: '0'
Track: schema-governance
Implements: RFC-8
Blocked-on: —
---
# 44 — Implement suggested_action on every finding

## What

Add `suggested_action` to the `Finding` struct and populate it for at least the rules where a fix is mechanically obvious (e.g. `field.quality`'s blank/placeholder cases), matching the output contract SPEC-2 already documents.

## Why

SPEC-2's own contract and RFC-8 both already describe `suggestedAction` as part of the JSON shape -- checked directly against `report.rs`'s real `Finding` struct, it isn't there. Decided, never built; a finding today says what's wrong but never what to do about it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): `SPEC-2` documents `suggestedAction` on a finding and the `Finding` struct lacks it; the structured-output gap is cheapest to close while few consumers exist, and it touches every rule constructor, so it belongs before more rules land. | **substantive** |
