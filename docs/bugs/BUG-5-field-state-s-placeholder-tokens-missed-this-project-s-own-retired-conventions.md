---
Status: Fixed
Stable-Id: 01M1ZF57J080NMEH2FZ6W6E044
Found-in: planning MILE-79 (a guarded edit command), reasoning through why the ADR-38 incident (Author silently reverted to a placeholder, undetected until a much later corpus-wide check) wasn't caught by field.quality despite field.quality running on every check
Regression-test: retired_project_placeholder_conventions_are_placeholder_not_present, field_quality_flags_a_retired_placeholder_as_an_error (rust/crates/urzua-core/src/field_state.rs, rust/crates/urzua-core/src/rules.rs)
Realized-by: code:rust/crates/urzua-core/src/field_state.rs, test:rust/crates/urzua-core/src/field_state.rs, test:rust/crates/urzua-core/src/rules.rs
---
# 5 — field_state's placeholder tokens missed this project's own retired conventions

## What was wrong

`field_state::classify`'s `PLACEHOLDER_TOKENS` list only covered the generic, unedited template
text (`"name"`, `"yyyy-mm-dd"`, `"tbd"`, `"todo"`) -- it never included `(project lead)`/`(session
author)`, this project's own placeholder convention used before MILE-78's identity backfill. A
field holding that exact text classified as `FieldState::Present`, indistinguishable from a real
name. So when `Author` on ADR-38 was hand-edited back to `(project lead)` this session, `field.quality`
-- which runs on every `check`, required for `adr`'s `Author` field -- had no way to flag it, and the
regression sat undetected until a much later, unrelated corpus review found it by eye.

## Why nothing caught it

A test locked the bug in as intended behavior: `a_real_value_that_contains_a_placeholder_substring_
is_not_flagged` asserted `classify(Some("(project lead)")) == FieldState::Present`, written (correctly,
at the time) to prove a real value merely containing a placeholder substring shouldn't be flagged --
but `(project lead)` was never actually a "real value example," it was this project's own live
placeholder text, and nobody revisited that test once the backfill made the string retired rather
than current. A test asserting a specific value's classification needs to be re-examined when that
value's own status in the corpus changes, not just left as a historical snapshot.

## References

- `field_state.rs`'s `PLACEHOLDER_TOKENS` -- the fix, plus the corrected test.
- MILE-78 -- the identity backfill that retired `(project lead)`/`(session author)` as live values,
  making them recognizable placeholders rather than merely "another string."
- ADR-38 -- the record whose `Author` regression this bug explains, though the fix lives here, not
  as an ADR amendment.
- MILE-79 -- the milestone whose planning surfaced this; `urzua edit`'s design remains separate and
  unbuilt, since this fix (not a new write path) is what actually closes this incident.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: added `(project lead)`/`(session author)` to `PLACEHOLDER_TOKENS`; corrected the test that had asserted the bug as intended behavior. **Why:** found live while planning MILE-79 -- the actual write-path design wouldn't have prevented this specific incident, but strengthening the classifier `field.quality` already runs unconditionally does. | **substantive** |

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
