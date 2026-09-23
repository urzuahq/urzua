---
Stable-Id: 01M36S8ZRSNACZH1K3JKRPY02P
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 24"
Regression-test: "not yet written -- Status: Open, no fix decided yet"
---
# 133 — is_record_reference and parse_record_filename independently re-implement the same record-identifier grammar

## What was wrong

`is_record_reference` (`rules.rs`, scans prose/pointer-field values for record mentions) and
`parse_record_filename` (`new_record.rs`, parses a filename into prefix + number) both implement "is
this a valid record identifier" -- hyphen-segment splitting, an all-digit number segment, uppercase
(and, since `BUG-114`, digit-bearing) prefix segments. `is_record_reference`'s own comment already
says the two "must agree," but nothing enforces it.

`BUG-114`/round 23 already found and fixed one instance of these two drifting apart (a digit-bearing
prefix). The underlying duplication that let that drift happen in the first place is still there.

## Why this needs a decision, not a quick extraction

The two functions don't operate on the same input shape: `parse_record_filename` handles a filename
stem specifically (date-heuristic exclusion, ambiguous-prefix-vs-number resolution via `at`'s
first-digit-segment scan, ADR-36's legacy-shape rejection), while `is_record_reference` handles a bare
reference token with no filename-specific context. A literal shared function would either need to
absorb `parse_record_filename`'s filename-specific logic into the reference recognizer (wrong -- a
reference token is not a filename) or extract only the common prefix/number grammar into a helper both
call, changing both functions' structure. Worth doing, not obviously a small edit.

## References

- `BUG-114` -- the specific divergence already found and fixed once; this bug is the general
  duplication that produced it.
- `BUG-111` -- the earlier hyphenated-prefix instance of the same two recognizers disagreeing.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review, immediately after fixing a live instance of the exact drift this duplication produces (`BUG-114`). Left `Status: Open` -- extracting a shared grammar needs deciding its shape, not a quick copy-paste fix. | **substantive** |
