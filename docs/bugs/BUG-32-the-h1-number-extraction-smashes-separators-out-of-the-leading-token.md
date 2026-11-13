---
Stable-Id: 01M2MF5THK4D6QD6BG2HVK0WCV
Status: Fixed
Found-in: 'reviewing the H1 number extraction while fixing BUG-23 -- `# 2026-09-16 Notes` yields the number 20260916'
Realized-by: test:rust/crates/urzua-core/src/rules/filename.rs#a_date_or_version_title_is_not_an_id_observed_failing@sha256:56c9d696510b994f979b9b697d424bab79582415ace17039ae62e4fa58a567e3, test:rust/crates/urzua-core/src/rules/filename.rs#a_title_number_too_large_to_compare_is_reported_as_overflow_not_as_absent_observed_failing@sha256:6b225f521ea62837bc3e103722f7c5735c5e291f91922459a95f65dbe188dc18, test:rust/crates/urzua-core/src/rules/filename.rs#a_declared_pattern_reads_the_title_id_as_its_prefix_and_a_number@sha256:1b0a0b7a70e4a25ee70ddabea1bedad9cf89fefbc30814fa20decf7252b637e7
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

## Decision (2026-11-05)

The maintainer decided: an H1's id is read with the type's declared `identity` pattern, the way file names are read (`MILE-151`). A title that does not match is reported as not carrying an id, never mangled; `# 2026-09-16 Notes` must not read as 20260916. A type with no pattern keeps a strict read, a leading `PREFIX-N` or a bare `N`. An overflowing number is reported as such.

Fixed in `filename.rs` (`h1_id`): the token is read as the declared `prefix`, a hyphen and ASCII digits. With no pattern, a leading `PREFIX-N` (an upper-case prefix, hyphenated if the type's is) or a bare `N` is the id, so a date such as `2026-09` is not one. A trailing `.` or `:` is punctuation. The number is compared by value, so padding never mismatches, and a number past `u64` is reported as too large to compare instead of as carrying no number. `SPEC-2` and `SPEC-3` state it.

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
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. `first_h1` (`rules/filename.rs`) still builds the number with `.chars().filter(char::is_ascii_digit)`, so `# 2026-09-16 Notes` yields `20260916`. **Why:** the shape a leading token must have is the decision the record names, tied to `RFC-29` and `RFC-44`. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** maintainer decision 2026-11-05: read the H1 id with the declared identity pattern, the way file names are read; a non-matching title carries no id; no pattern keeps a strict `PREFIX-N` or `N` read; overflow is reported as such. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
