---
Stable-Id: 01M2QB6R7QKVP0DTWETQAG7ZFR
Status: Fixed
Found-in: 'MILE-13 -- `Blocked-on: "RFC-9''s own Q2"`, a real dependency in this repository that no rule can see'
Regression-test: 'rust/crates/urzua-core/src/rules.rs::a_possessive_does_not_hide_a_reference_observed_failing -- RFC-9''s resolves, RFC-9a and RFC-9s stay unrecognised, and trailing punctuation still works'
---
# 39 — `narrative-field.stale` silently misses a reference with punctuation attached

## What is wrong

`narrative-field.stale` scans declared narrative fields for references to records and reports when a
referenced record reaches a terminal status. It cannot see a reference with anything attached to it.

`MILE-13` declares `Blocked-on: RFC-9's own Q2`. Both extractors return nothing:

| extractor | on `"RFC-9's own Q2"` | why |
|---|---|---|
| `extract_references` | `[]` | reads the first whitespace token of each comma-separated entry -- here `"RFC-9's"` -- then rejects it, because `9's` is not all digits |
| `scan_references` | `[]` | trims non-alphanumerics from each end, and `s` is alphanumeric, so the possessive stays attached |

So a real dependency -- `MILE-13` is blocked on `RFC-9` -- is invisible, and the rule reports success
by examining a field it could not read.

## Why it matters

This is the failure direction this project treats as the worse one: not a wrong answer, an answer
that never arrives. `RFC-9` is `Draft` today, so nothing is stale yet and the gap is costless. The day
it is accepted, `MILE-13` will not be flagged, and the only evidence will be its absence.

Found the same way twice in one session. `claim.status-agreement` initially used `extract_references`
on prose and returned nothing, looking exactly like a rule that had run clean.

## Fix

Strip a trailing possessive and trailing punctuation before deciding whether a token is a reference.
Narrow deliberately: `RFC-9's` and `RFC-9,` and `RFC-9.` are the same reference, while `RFC-9a` is not
a reference to `RFC-9` and must stay rejected.

Worth a paired test rather than a single case -- the two extractors already disagree about what a
reference looks like, and a third notion of it is how they drift further apart.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-17 | Filed. **Why:** found while deciding whether `MILE-13` needed its `Blocked-on` prose converted by hand. Converting it would have hidden the defect -- the record is one instance, and the rule under-reports for every reference written with punctuation attached. | **substantive** |
> | 2026-09-17 | `Status: Open` → `Fixed`. **Why:** both extractors now strip a possessive before deciding whether a token is a reference, and only a possessive -- `RFC-9a` and `RFC-9s` stay unrecognised, because a suffixed identifier is a different record. Observed failing first (`left: []`). Verified against the live instance rather than a fixture: `narrative-field.stale` went from examining 7 narrative fields to 8, and flipping `RFC-9` to `Accepted` produces the finding on `MILE-13` that this record predicted would never arrive. | **substantive** |
> | 2026-09-19 | `Regression-test` now names the test that exists. **Why:** the field still read *"not yet written"* after the test was written and observed failing, so this record claimed the work was undone while the work was done. One of eight such records, found by the audit that filed `BUG-57`. | **structural** |
