# 44 — Implement suggested_action on every finding

> Status: Planned
> Stable-Id: 01M1YN83N5WHAE63XN53HTDGHR
> Phase: 1
> Track: schema-governance
> Implements: RFC-8

## What

Add `suggested_action` to the `Finding` struct and populate it for at least the rules where a fix is mechanically obvious (e.g. `field.quality`'s blank/placeholder cases), matching the output contract SPEC-2 already documents.

## Why

SPEC-2's own contract and RFC-8 both already describe `suggestedAction` as part of the JSON shape -- checked directly against `report.rs`'s real `Finding` struct, it isn't there. Decided, never built; a finding today says what's wrong but never what to do about it.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
