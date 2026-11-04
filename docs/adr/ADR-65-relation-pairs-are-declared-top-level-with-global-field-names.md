---
Stable-Id: 01M6J0TMERA6FQ26EE3Q8HWACH
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-47
---
# 65 — Relation pairs are declared top-level, with global field names

## Context

`RFC-47` proposed that a repository declare which relation fields are inverses of each other, so reciprocity
is checked for any declared pair rather than for the one combined `Supersedes / Superseded-by` key the engine
hardcodes, and left several questions open. `MILE-122` built the narrowest reading of each. This ADR records
those readings so they are decided, not merely implemented.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| Where a pair is declared | Top-level `relation_pairs` vs. per record type | A pair spans types (an ADR resolves a bug); a per-type block would declare one pair twice and let the two copies disagree |
| Field naming | Global names vs. per-type binding of a pair's sides | Global names are checkable at load against the fields types declare; a per-type binding is a second vocabulary nothing yet needs |
| Combined keys (`A / B` in one header key) | Support vs. leave unsupported | `RFC-47` leaves it open and the old rule still covers the one combined key that exists |
| `reciprocity: advisory` | Accept vs. refuse at load | The rule's configured level is already the severity (`MILE-80`), so a per-pair advisory would set nothing; it is refused, not read as `required` |
| Population unit | Records vs. field slots | A record can carry both sides of a pair; the unit is one `(record, pair, side)` slot where the type declares the field, so the census counts what was actually judged |
| `Amends` one-way | Declare it one-way in the tool vs. leave to each corpus | Which pairs are one-way by design is a corpus's fact; a pair declared `reciprocity: none` records direction without checking echo |
| The old `relation.supersession-reciprocity` | Replace vs. keep beside | Replacing changes verdicts for every adopter using it; it stays, and the new rules sit beside it |

## Decision

In the context of `RFC-47`, facing reciprocity hardcoded to one combined key, we decided to add a top-level
`relation_pairs` list (`name`, `forward`, `inverse`, `reciprocity: required | none`, `cross_type`, default
`true`) with two rules, `relation.pair-reciprocity` and `relation.cross-type-disallowed`. Field names are
global to the repository; combined keys are unsupported; `reciprocity: advisory` is refused at load; the
population unit is the field slot; whether `Amends` is one-way is left to each corpus's own declaration; and
`relation.supersession-reciprocity` is kept unchanged. A misdeclared pair, or a pair rule enabled with no pairs
declared, is a config error. We accept that a per-type spelling of the same relation needs two pairs.

## Reversibility

Moderate. The config key and rule ids are public once released; adding per-type binding or combined keys
later is additive, while removing `relation_pairs` would break any config that adopted it.

## Consequences

- Any inverse pair, including across record types, is checkable, and a cross-type claim a pair forbids is
  reported by its own rule rather than skipped.
- Two pairs cannot share a field, so a field has exactly one partner.
- An unresolved target is reported by `relation.pair-reciprocity` as well as by `pointer.resolution`; an
  unreadable target is disclosed under its own entry, not blamed on the citer.
- Two rules now run beside the older one for the `Supersedes / Superseded-by` case when a repository also
  declares it as a pair.

## References

`RFC-47`, `MILE-122`, `ADR-60`, `ADR-63`, `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed. **Why:** `RFC-47` is being built by `MILE-122`, and building part of an RFC needs its own decision record; the maintainer approves before it is Accepted. | **substantive** |
