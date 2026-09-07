# 75 — Header sub-format consistency within a record type

> Status: Planned
> Stable-Id: 01M1YWKN2HHC7DW4Y422DWJK1F
> Phase: 1
> Track: schema-governance
> Implements: RFC-10, ADR-8

## What

`HeaderShape::Blockquote` deliberately tolerates three sub-formats interchangeably (one-field-per-
line, bold-labelled, single-line pipe-delimited) so field extraction is robust to real-world
variance (RFC-10). That tolerance means `check` has no way to flag that SPEC-1 renders as multi-line
plain-label blockquote while SPEC-2 through SPEC-6 render as single-line pipe-delimited bold-label
blockquote — both parse correctly, so nothing fires, even though every other spec in the corpus
agrees on one visual family and SPEC-1 doesn't. Decide whether within-type sub-format drift deserves
its own (likely `Warning`-severity) rule, and if so, build it; if not, say explicitly why format
tolerance across sub-shapes is permanent rather than a maturity gap, the same way SPEC-1 already
does for structural-vs-content-scope.

## Why

Found live, reviewing SPEC-1 against SPEC-2 through SPEC-6: SPEC-1 has a `Version` field and
`check` reads it fine, but its layout has visibly drifted from every other spec in the corpus and
nothing says so. RFC-10's own three-shapes-recognized design was built to stop real corpus variance
from breaking field extraction — a reasonable goal — but as a side effect it also means the tool
can't tell a maintainer "these look inconsistent," which is a legitimate part of what "header
format consistency" (rules.rs's own doc comment for Rule 1) implies to a reader even though the
implementation only checks required-field presence per a declared shape, not sub-format uniformity.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
