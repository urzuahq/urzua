---
Stable-Id: 01M2MF5THK4D6QD6BG2HVK0WCV
Status: Open
Found-in: 'reviewing the H1 number extraction while fixing BUG-23 -- `# 2026-09-16 Notes` yields the number 20260916'
Regression-test: 'not yet written -- needs a case per shape (`2026-09-16`, `v1.2`, a 20-digit run) asserting the extractor either reads a record number or reports that it could not, observed failing against the current filter-all-digits implementation'
---
# 32 — The H1 number extraction smashes separators out of the leading token

## What was wrong

The leading token of an H1 is reduced to a number by deleting every non-digit character from it:

```rust
text.split_whitespace().next().unwrap_or_default()
    .chars().filter(char::is_ascii_digit).collect()
```

That is right for `SPEC-0001` and `36` — the shapes it was written for. It generalises badly. A
date-titled record, `# 2026-09-16 Notes`, yields `20260916` and is then reported as *"the H1 title
claims 20260916"*. The number is not merely wrong, it was never present.

Date-titled and version-titled records are ordinary in the corpora this tool courts — incident
reviews, meeting notes, changelog-style entries.

A second, quieter case shares the cause: a digit run too large for `u64` parses to `None` and is
reported as *"carries no number"* when the title plainly carries one. Two different failures
collapsed into one message — the same shape as `BUG-23`, one level down.

## Why nothing caught it

Every H1 in this corpus is `NN — Title` or `TYPE-NN — Title`, so the token is digits plus at most a
type prefix and the filter is indistinguishable from correct parsing. Nothing constructs a leading
token with internal separators, and no test asserts what the extractor should do with one.

## Deliberately not fixed with BUG-23

`BUG-23` is a message-quality fix in the same function, and folding a parsing change into it would
mean two behaviour changes under one record. The fix here is a real decision — whether a leading
token must match a record-number shape (`^(?:[A-Z]+-)?\d+$`) rather than merely contain digits, and
what the rule reports when it does not.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `first_h1`, the digit filter.
- BUG-23 -- the message-collapse defect in the same function, fixed separately.
- RFC-29 -- the related question of whether the H1 numbering convention should be declarable at all;
  if it is, this extractor's contract changes with it.
- RFC-44 -- proposes the actual fix: an `identity` field type as part of a general declared
  field-shape mechanism, closing this defect for this repository's own record types (RFC-29's
  foreign-corpus case stays open).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Initial record, `Status: Open`. Not fixed -- it needs a decision about what shape a leading token must have, not a wording change, and `BUG-23`/`BUG-29`/`BUG-30` already change this function once. | **structural** |
