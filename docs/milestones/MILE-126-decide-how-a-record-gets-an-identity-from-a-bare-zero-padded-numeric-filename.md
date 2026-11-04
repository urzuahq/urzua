---
Stable-Id: 01M6CXZXSNMN0AYVBVYEWRY95J
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: spec:docs/adr/ADR-36-filenames-carry-the-type-prefix-going-forward.md
---
# 126 — Decide how a record gets an identity from a bare zero-padded numeric filename

## What

Decide how a record gets an identity from a bare zero-padded numeric filename such as `0001-a.md`, and
how a `prefix` declared on the type relates to it. Reproduced on a two-record corpus with
`0001-a.md` and `0002-b.md` in `docs/adr`, a config declaring `prefix: ADR`, and
`Supersedes: ADR-1` / `Superseded-by: ADR-2`: `urzua audit` reports both `pointer.resolution` targets as
"does not resolve to any discovered record". Declaring a prefix does not give bare-number files an
identity. Prose and code in such corpora cite the zero-padded form, `ADR-0173`, which the decision must
also say how to resolve.

## Why

A real adopter corpus of several hundred records uses exactly this filename shape, so every cross-record
pointer in it fails to resolve until this is decided; the first `check` is wrong in the same silent way
`BUG-99` found for `audit`. `ADR-36` as amended accepts only type-prefixed filenames, and a reference to a
bare-number filename is dangling; this milestone decides whether that policy should change for
corpora already in this shape. `MILE-98` plans the declared identity pattern that could express the
change. The
open question is whether a prefix on the type is enough, whether the identity pattern is the only
route, and how `ADR-1` and `ADR-0001` relate. The reproduction was recorded when this was filed; re-run it
when the decision is taken.

## References

- `BUG-99` -- `audit` exits 2 on the config `init` writes for a corpus with no type prefix.
- `MILE-98` -- the declared document model, including identity patterns.
- `ADR-36` -- filenames carry the type prefix going forward.

## Decision (2026-11-02)

`RFC-46` was rejected: a bare-number filename does not get an identity from its type. Such a corpus is renamed to `TYPE-NNNN-slug.md` (`ADR-36`), and zero-padded citations such as `ADR-0173` keep resolving because IDs are compared numerically. `MILE-120` reports the mismatch with rename guidance; `MILE-134` is the later rename tool. Closing this milestone is a human decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A declared prefix does not give bare zero-padded numeric filenames an identity, so a real adopter's pointers all fail to resolve; filed as a decision. | **substantive** |
> | 2026-11-02 | Decision recorded from `RFC-46`'s rejection. **Why:** The question this milestone asked is now answered. | **substantive** |
> | 2026-11-04 | `MILE-98`'s declared `identity` pattern must capture the type prefix, so a bare zero-padded filename (`0001-a.md`) still gets no identity and the decision stands: such a corpus is renamed to `TYPE-NNNN-slug.md` (`ADR-36`), where `ADR-0173` resolves against `ADR-173` numerically. A pattern without a `(?P<prefix>...)` group fails the config load. **Why:** `RFC-46` was rejected; the declaration is not a back door to the bare-number identity it declined. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged on `main`: the decision is recorded in the revision log (a bare zero-padded filename gets no identity; such a corpus is renamed to `TYPE-NNNN-slug.md`) and in the accepted `ADR-36`; `RFC-46` was rejected. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
