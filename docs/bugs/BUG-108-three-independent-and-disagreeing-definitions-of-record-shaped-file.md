---
Stable-Id: 01M33S5SSE41F78KF9K703078X
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Realized-by: test:rust/crates/urzua-core/src/record.rs#a_leading_underscore_is_a_template_not_a_record@sha256:d1c6faf967a168ecd73bf4a0c4a011c8a63908e97bfeb07d908438686c270533, test:rust/crates/urzua-core/src/record.rs#a_non_markdown_file_is_not_a_record@sha256:a5774fdc859adf387fec5027e40aaebb0212e73e86849df519e457c5f6983cb8, test:rust/crates/urzua-core/src/record.rs#an_unnumbered_markdown_file_is_still_governed@sha256:b7ff44035f31edf0095e799687dee77eb38aca1f683b308a40df45b2167ec25f
---
# 108 — three independent and disagreeing definitions of record-shaped file

## What was wrong

`discovery.rs:81` (`load_records`) and `rules.rs:378`
(`type_record_outside_declared_dir`'s eligibility filter) each inlined the identical test —
`!file_name.starts_with('_') && file_name.ends_with(".md")` — verbatim, in two different crates.
`init.rs`'s `is_record_shaped` used a third, stricter test deferring to
`new_record::parse_record_filename`, which additionally requires a parseable number.

The two identical copies were a real duplication risk (`RFC-39`'s own argument, one level up from the
values it names): a future change to one — say, excluding a second template convention — would need to
be remembered at the other call site, with nothing to enforce it.

## Why nothing caught it

Each site was written correctly for its own purpose at the time, and nothing compares call sites across
crates. The literal duplication between `discovery.rs` and `rules.rs` had no test proving they agree,
only the fact that both were copy-pasted from the same original.

## What changed

`is_governed_record_filename` (`record.rs`) is now the one definition `discovery.rs` and `rules.rs`
both call. `init.rs`'s `is_record_shaped` stays separate, with a comment explaining why: it answers a
different question — "is this evidence of the corpus's numbering convention" — that `check`'s loading
and governance never ask. Unifying it into the loose form would let adopt mode start proposing a type
from directories holding only numberless files it could never assign a next number in; unifying the
loose form into the strict one would silently drop every numberless file `check` currently governs.
Both are correct answers to different questions, not a third guess.

## References

- `RFC-39` — makes the general argument (a comparison duplicated across call sites drifts) this bug is
  a concrete instance of, one level up (a predicate rather than a value comparison).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; the fix was a mechanical extraction with no design decision to defer, unlike `BUG-100`/`BUG-101`. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
