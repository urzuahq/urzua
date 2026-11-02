---
Stable-Id: 01M6CXZY4QSHAD89X81W3MW6SN
Status: Planned
Phase: '0'
Track: schema-governance
---
# 127 — Decide whether a relation claim on a not-yet-accepted record is held to a weaker standard than an accepted record's

## What

Decide whether a relation claim on a record that is not yet accepted (a `Supersedes` or `Superseded-by`
on a `Draft` or `Proposed` record) is held to a weaker standard than the same claim on an accepted
record: a provisional, pending channel in which the claim is recorded and visible but does not yet
assert that the target is superseded.

## Why

`AGENTS.md` says a `Supersedes / Superseded-by` value asserts a decision was reached, and is therefore
not the author's to set. But a proposal legitimately intends to supersede something before it is
accepted, and a corpus that cannot say so either leaves the intent in prose or makes a premature
assertion. Today `relation.supersession-reciprocity` treats both sides identically. An adopter
corpus will have in-flight proposals that name what they would supersede, so the answer decides whether
its records can be checked without lying about their state.

## References

- `relation.supersession-reciprocity` -- the rule that treats both sides identically today.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** Whether an in-flight proposal may name what it would supersede, without asserting it, is undecided and an adopter will hit it. | **substantive** |
