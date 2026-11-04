---
Stable-Id: 01M6N0R3T1R3DST4TUSB5G1LL1
Status: Open
Found-in: 'creating ADR-72 in this repository (2026-11-04), which made citation.resolution resolve ADR-18 and RFC-5 citations of ADR-72 to it; the waivers for them were removed in #256 because waiver.matches-nothing reported them'
Regression-test: 'pending'
---
# 190 — An illustrative citation of an id from another corpus silently starts resolving to an unrelated real record once that id exists

## What was wrong

`ADR-18` and `RFC-5` cite "ADR-72/73/74" as a worked example: three records from a different corpus that independently claimed the same evidence. The ids are illustration, not references. While this repository had no ADR-72, `citation.resolution` found them unresolvable and standing waivers covered the findings.

When this repository created its own `ADR-72` (allowed status transitions), the citation began to resolve, silently, to an unrelated real record. The finding disappeared, so each waiver matched nothing and `waiver.matches-nothing` reported it; the waivers had to be removed in #256. Nothing was wrong with the waivers' intent. The citations now point at a record that has nothing to do with the example, and no rule says so.

The same will happen when the next id in the sequence (in draft as #260) and the one after it land. Any record that cites an id as history or illustration is exposed the same way, and the exposure is permanent: every new record in the namespace can capture an old illustrative citation.

## Why nothing caught it

`citation.resolution` has two states for a cited id: resolves, or does not. A waiver can excuse the second but cannot say that a citation must never be treated as the first. `external_ids` exempts an id repository-wide, which is wrong here: `ADR-72` is a real record elsewhere in this corpus and must keep resolving when it is cited as a reference. Nothing lets one record declare that one of its own citations is an illustration. A resolved illustrative citation also looks like a correct one, so the failure is silent and needs a person to notice the stale waiver.

## Proposed approach

Add a config-declared way to mark a citation as illustrative so that it is never resolved, is not counted as a pointer, and is reported if it ever would have matched a real record (so the author learns the example has become ambiguous rather than silently drifting). The two shapes to decide between:

- a declared inline marker, for example `illustrative_citations: {marker: "e.g."}` or a declared delimiter pair around the citation, read from the prose around the id;
- a per-record list, a declared header field (for example `Illustrative-ids` listing the three example ids) naming ids the record cites as examples, so the record states it once and the engine skips those ids in that record only.

The per-record list is the stronger candidate: it is explicit, scoped to one record, needs no prose parsing (ADR-64: structured config in, structured output out), and a declared id that matches nothing in the record is itself reported (no silent no-ops). The type's `known_fields` and the field name are declared in config, never hardcoded. This needs a decision on the shape before it is built; the maintainer decides.

## Done means

- A record can declare, through config, that a citation of a given id is illustrative, and `citation.resolution` neither resolves nor reports it, in that record only.
- A declared illustrative id that the record does not cite is reported, not ignored.
- The same id cited as a real reference in another record still resolves.
- `ADR-18` and `RFC-5` declare all three example ids as illustrative, and `urzua check` on this repository reports nothing for them, with no waiver, before and after the other two ids exist.
- `SPEC-2` and `SPEC-3` document the declaration with revision rows; the regression test names the incident (an illustrative id later created as a real record) and is observed failing first.

## References

- `ADR-18`, `RFC-5`: the two records that cite the example.
- `ADR-64`: structured config in, structured output out.
- #256: removed the waivers; #260: the next id in draft.
- `BUG-186`: the same class, a declared input the repository sets aside on purpose being misreported.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed at Phase 0. **Why:** found while creating `ADR-72`; the same will recur for the other two ids. Left `Status: Open`: the declaration's shape needs a maintainer decision. | **substantive** |
