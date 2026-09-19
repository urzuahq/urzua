---
Stable-Id: 01M2WG83X4FDF5ADVSCPF7P434
Status: Open
Found-in: "Round 7 of the 0.4.0 review"
Regression-test: "not yet written -- every rule init proposes must examine a non-zero number of records on the corpus init adopted"
---
# 73 — init proposes rules whose declarations it did not write so they examine zero records

## What was wrong

`init` writes `required_fields: []` and no `known_fields`, `header_layout` or `pointer_fields`, then
proposes `field.quality`, `field.pending`, `header.field-set-consistency`,
`header.layout-consistency` and `header.pointer-field-clean` at `warn`.

All five read a declaration `init` did not write. On an adopted corpus each reports
`status: ran, records_examined: 0` -- structurally incapable of firing given the configuration `init`
itself produced.

`init` already declines to propose a rule whose required *options* it cannot supply, and (after
`BUG-70`) a rule whose identity assumption the corpus does not satisfy. The same judgement extends to
a rule whose required *declaration* `init` left empty: either propose the rule and the declaration it
needs, or do not propose the rule.

Filed rather than fixed with the rest of round 7: which declarations `init` should infer from a
corpus, versus leave to the adopter, is a design question and `ADR-33` already decided that adopt
mode proposes where to grow rather than preserving what it found.

## Why nothing caught it

Adopt mode's tests assert on the configuration `init` writes. None runs `check` with that
configuration and asserts the proposed rules reached a verdict. A config that is well-formed and
inert passes every test there is.

## References

- `BUG-61` and `BUG-70`, the same judgement applied to options and to identity.
- `ADR-33`, on what adopt mode proposes.
- `MILE-51`, the adopt-a-foreign-corpus validation.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
