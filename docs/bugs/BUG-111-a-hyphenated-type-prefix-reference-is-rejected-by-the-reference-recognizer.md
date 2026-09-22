---
Stable-Id: 01M34MM5FGK8TZ7FDD8Y1EYGRX
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, reviewing everything landed for the 0.4.0 release"
Regression-test: "a_hyphenated_type_prefix_is_a_recognized_reference_observed_failing, rules.rs"
---
# 111 — a hyphenated type prefix reference is rejected by the reference recognizer

## What was wrong

`parse_record_filename` supports a hyphenated type prefix (`DOC-ADR-2-first-thing.md` parses as
prefix `DOC-ADR`, number `2` — a real, tested shape). `scan_references`, `extract_references`, and
`header_pointer_field_clean`'s clean-reference check all recognized a reference by splitting on only
the *first* hyphen: `token.split_once('-')` on `"DOC-ADR-2"` yields `prefix="DOC"`, `num="ADR-2"`, and
`num` fails the all-digit check, so the whole token was rejected as not-a-reference.

A record created with a hyphenated prefix could therefore never be correctly referenced: a claim
citing it (`claim.status-agreement`), a pointer or narrative field naming it
(`pointer.resolution`/`narrative-field.stale`), and a clean-format pointer field
(`header.pointer-field-clean`) all either silently treated the citation as absent or flagged it as
malformed — three independent rules, one root cause.

## Why nothing caught it

The three call sites duplicated the same single-hyphen-split predicate rather than sharing one
definition, and nothing exercised a reference to a hyphenated-prefix record — only
`parse_record_filename`'s own test (`a_hyphenated_type_prefix_parses_observed_failing`) covered the
prefix shape at all, and that test is about parsing a *filename*, not recognizing a *reference* to one.

## What changed

One shared predicate, `is_record_reference`, splits on every hyphen and treats the *last* segment as
the number and everything before it as the prefix (matching `parse_record_filename`'s own algorithm).
All three call sites use it.

## References

- `parse_record_filename` (`new_record.rs`) — the filename-parsing counterpart whose supported shape
  this bug's reference-side recognizer failed to match.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test observed failing against the single-hyphen-split predicate before the fix. | **substantive** |
