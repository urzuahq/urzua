---
Stable-Id: 01M26CKB9RQXDTJ7PP53THBQ1F
Status: Fixed
Found-in: 'surfaced while scoping a new RFC to amend RFC-2 -- asked why `Amends` has no `relation.supersession-reciprocity`-style check the way `Supersedes / Superseded-by` does, and found no ADR or RFC ever actually decided the asymmetry either way'
Realized-by: test:rust/crates/urzua-core/src/rules/relation.rs#an_inverse_claim_the_forward_side_does_not_echo_is_a_finding@sha256:4b7a28c106941bb21338aa8b220ba2c9c2eb06d7378e6d246e382451b9901955, test:rust/crates/urzua-cli/tests/provisional_claim_integration.rs#an_accepted_amendment_the_peer_has_not_echoed_is_an_error@sha256:51ca8c61a9ff08a6df2928883e0fbc13b6de005fa9d3ac983a877c10885c69d2
---
# 17 — Amends has no reciprocal field or reciprocity check, unlike Supersedes / Superseded-by

## What was wrong

`Supersedes / Superseded-by` is a single combined field, checked for mutual reciprocity by
`relation.supersession-reciprocity`: if A names B, B must name A back, or it's a finding. `Amends`
is a plain one-directional pointer -- existence-checked by `pointer_resolution` like any other
`pointer_fields` entry, but nothing requires (or even offers a field for) the amended record to
point back. A reader landing on the base record (e.g. `RFC-10`, amended by `RFC-16`) has no
mechanical way to discover that a correction exists, unlike a superseded record, which is guaranteed
to signpost its replacement.

`RFC-1` (this project's own original schema proposal, line 210) lists `supersedes` and `amends`
together as the same category of relationship field -- suggesting they were conceived as siblings —
but only one of the two ever got the reciprocal-field-plus-reciprocity-check treatment. Grepping the
entire corpus for any ADR or RFC that explicitly decided `Amends` should stay one-directional found
nothing: the asymmetry isn't a documented design choice, it just happened, because `Amends` was
added later, driven by a single real case (`RFC-16` amends `RFC-10`), without the same design pass
`Supersedes / Superseded-by` got.

## Why nothing caught it

Only one real `Amends` instance exists in this corpus (`RFC-16` → `RFC-10`), and nobody has yet hit
the scenario of landing on `RFC-10` wanting to know it's been amended and not finding out
mechanically -- the gap has had no real cost yet, only a hypothetical one. A schema-level asymmetry
like this doesn't produce a `check` finding of its own; it's the kind of thing only surfaces by
someone directly asking "why does X work differently from Y" and checking whether that was ever a
real decision, which is exactly how this was found.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `supersession_reciprocity`, the existing mechanism this
  bug asks whether `Amends` should mirror.
- `docs/specs/SPEC-17-the-rfc-record-type.md` -- `Amends`'s current field definition.
- RFC-1 -- groups `supersedes`/`amends` together as one category of relationship field, the evidence
  this asymmetry was never a deliberate split.
- RFC-16 -- the one real `Amends` instance in this corpus.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- whether the right fix is a reciprocal `Amended-by` field, a reciprocity check reusing the existing combined-field pattern, or a deliberate decision that one-directional is correct after all (and documenting that decision explicitly, closing this without a code change) is not yet decided. | **structural** |
> | 2026-11-04 | `relation_pairs` (`MILE-122`) now lets a repository declare `Amends` / `Amended-by` and have it echoed; this repository declares no pair, so the finding set is unchanged. Whether `Amends` should be one-way here is still undecided, so `Status` and the fix scope stand. **Why:** the general mechanism the fix needed now exists. | **substantive** |
> | 2026-11-04 | `Status` set to `Fixed`; `Regression-test` filled. Fixed in #163 (`MILE-122`: declared `relation_pairs` and a general reciprocity rule); this repository declares the `amendment` pair (`Amends` / `Amended-by`, `reciprocity: required`, `rfc` only) in `.urzua/config.yaml`, with a provisional level while the claimant is a Draft (`MILE-155`, `MILE-163`). **Why:** `Amends` is now checked for a reciprocal field the way `Supersedes / Superseded-by` is, and the pair is declared per repository rather than hardcoded to one relation. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
