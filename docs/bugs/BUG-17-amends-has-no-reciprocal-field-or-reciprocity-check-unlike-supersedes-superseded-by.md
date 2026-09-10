---
Stable-Id: 01M26CKB9RQXDTJ7PP53THBQ1F
Status: Open
Found-in: 'surfaced while scoping a new RFC to amend RFC-2 -- asked why `Amends` has no `relation.supersession-reciprocity`-style check the way `Supersedes / Superseded-by` does, and found no ADR or RFC ever actually decided the asymmetry either way'
Regression-test: 'not yet written -- fix scope (reciprocal field vs. reciprocity-only check, one-directional-by-design vs. genuinely fixed) not yet decided'
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
