---
Stable-Id: 01M6JKNT9WX2Y4PVJYEMKKZQ6X
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 155 — Relation pairs declared per type so a pair between specs does not error on a type that cannot carry the inverse

## What

Let a relation pair be bound to the record types that carry it, so a pair declared between one type does not error on a claim to a type that cannot hold the inverse field.

## Why

`relation_pairs` are global. A pair declared between specs errors on a claim to a type with no inverse field, and `cross_type: false` only moves the error to another rule. SPEC-3 describes per-type binding as none, which is the gap; the rule cannot express a pair that exists only for some types. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A pair can name its carrying types; a claim to an uncarrying type is judged by that scoping rather than erroring; the change is recorded in the spec that owns relation pairs; a fixture covers a scoped pair and a global one; and the capability is exercised by urzua's own `.urzua/config.yaml` and records in this repository, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed this capability gap; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
