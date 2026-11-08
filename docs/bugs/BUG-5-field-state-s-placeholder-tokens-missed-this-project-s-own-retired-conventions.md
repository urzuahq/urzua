---
Status: Fixed
Stable-Id: 01M1ZF57J080NMEH2FZ6W6E044
Found-in: planning MILE-79 (a guarded edit command), reasoning through why the ADR-38 incident (Author silently reverted to a placeholder, undetected until a much later corpus-wide check) wasn't caught by field.quality despite field.quality running on every check
Realized-by: code:rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e7dbcfee70b6c994f022fb02a22b138bea50167028e46c232ceed, test:rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e7dbcfee70b6c994f022fb02a22b138bea50167028e46c232ceed, test:rust/crates/urzua-core/src/rules.rs@sha256:355dd5bf590d2020ab967cb30501aa6ce610186c6631f22aa828831a6fbac0d4, test:rust/crates/urzua-core/src/field_state.rs#retired_project_placeholder_conventions_are_placeholder_not_present@sha256:93bd89a306f7a4462d5e6d89612bb354976646abfd0502a4018b5934a54ae327, test:rust/crates/urzua-core/src/rules/field.rs#field_quality_flags_a_retired_placeholder_as_an_error@sha256:6ebe9d8381f273a3e0fd683f0f1fcefa7e281a7d1184d41ba953b9954eeef647
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
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e, rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e, rust/crates/urzua-core/src/rules.rs@sha256:355dd5bf590d Reason: Pinned the files as they are now after checking each against the record it evidences: publish-release.yml still builds and uploads the archives itself (BUG-28), new_record.rs still renders only the configured YAML header and new_integration.rs still holds the test that a blockquote template does not override it (BUG-3), waiver.rs is_active still parses Expires as a date (BUG-31), prepare-release.yml still hands knope's nothing-to-release exit to release-guard.sh instead of failing (BUG-33), ci.yml's changeset job still reads the label live (BUG-34), knope.toml still releases from changesets only (BUG-35), main.rs and check_integration.rs still emit doctor as JSON (BUG-4), release-guard.test.sh still plants an unmatched-package fragment and asserts a nonzero exit (BUG-48), field_state.rs still carries the retired placeholder tokens (BUG-5), and generate-rule-table.py and the Makefile rule-table-check target still compare SPEC-2's rule table with the shipped rule set (BUG-52). BUG-29 and BUG-30 are not in this batch because the filename.title-consistency code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
