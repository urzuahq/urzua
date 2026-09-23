---
Stable-Id: 01M36PD8JJ4SF9KQ53EYYET1V2
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 23"
Regression-test: "not yet written -- Status: Open, no fix decided yet"
---
# 130 — duplicate_keys exact-match extends ADR-57's declared-vocabulary decision to a type declaring no vocabulary at all

## What was wrong

`Header::duplicate_keys()` compares field keys exactly, citing `ADR-57`: *"`Status` and `status` are
two fields, so writing both is not this defect."* For a type with no `known_fields`/`required_fields`
declared at all, nothing else catches a `Status`/`status` repetition either:

- `header.field-set-consistency` only examines records of a type present in `known_fields_by_type`
  (`rules.rs`, `header_field_set_consistency`'s own candidate filter) -- an undeclared type is never a
  candidate.
- `header.required-fields`'s own duplicate-key check (`header_required_fields`) is gated the same way,
  on `required_by_type.get(&record.record_type)`.

So a header writing both `Status: Draft` and `status: Accepted` on such a type passes with zero
findings from either rule.

## Why this may not be as simple as reverting

`ADR-57`'s own context is specifically `header.field-set-consistency`'s declared-vocabulary matching:
deciding whether a header key names a field the type's config *declared*. Its reasoning ("a different
spelling is a different field," measured against real corpus data) is about matching against a
declared vocabulary, where a false case-insensitive match hides an undeclared field. `duplicate_keys()`
is a different concern: it detects the same author writing the same key twice in one document, which
has no comparable "maybe they meant two different fields" case for a type with no declared vocabulary
to disagree with -- nobody intentionally writes `Status` and `status` in the same header meaning two
separate, deliberate values.

Whether `duplicate_keys()`'s extension of `ADR-57` was a considered decision or an unexamined
consequence of applying "field names compare exactly" everywhere is unclear from the code and `ADR-57`
itself, which doesn't mention `duplicate_keys()`. Filed rather than reverted, since correcting it needs
deciding whether within-document duplicate detection is really the same question `ADR-57` answered, or
a distinct one that deserves its own comparison rule (possibly case-insensitive specifically for
`duplicate_keys()`, leaving `header.field-set-consistency`'s declared-vocabulary matching untouched).

## References

- `ADR-57` -- the decision `duplicate_keys()`'s doc comment cites, whose own context is
  declared-vocabulary matching, not within-document duplicate detection.
- `header.field-set-consistency`, `header.required-fields` -- the two rules that would have caught this
  for a type that *does* declare `known_fields`/`required_fields`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified the gap is real for a type declaring neither `known_fields` nor `required_fields`, and left `Status: Open` since fixing it means deciding whether `ADR-57`'s exact-match reasoning genuinely extends to this different concern, not just reverting a line. | **substantive** |
