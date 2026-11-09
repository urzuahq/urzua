---
Stable-Id: 01M2YN69PC0WBYK6ZWEBV9Z4D0
Status: Fixed
Found-in: "Round 8 of the 0.4.0 review"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#a_run_in_which_no_rule_examined_anything_is_not_ok@sha256:cf759d07934a2a418238cb6992bc974a0bbf7de105b4ecb69f7b1f019f9268f3, test:rust/crates/urzua-cli/tests/check_integration.rs#audit_with_no_rule_declared_is_not_ok@sha256:de028eec3b26c8c88ea56498ca69ca410559a97ebd3dafa9246d19e57a417c82
---
# 77 — check and audit report ok when no rule examined anything

## What was wrong

`check` and `audit` both compute `status` from how many records were in scope, never from whether
any rule looked at them.

```rust
let status = if examined.is_empty() { NotRun } else if active_findings == 0 { Ok } else { ... };
```

Reproduced in `check`: a config with `rules: {}` over one ADR missing its required `Status` gives
`status: ok`, `files_examined: 1`, zero rules with `status: ran`, **exit 0**. A configuration that
loses its `rules:` block in a merge is indistinguishable from a clean corpus.

Reproduced in `audit`: gating its two rules through the shared table (`BUG-42`) created a path where
neither is declared. `audit` then executes nothing and reports `status: ok`, `files_examined: 1`,
both rules `not-enabled`, **exit 0**. Before that change both rules always ran, so the path did not
exist.

This is the pattern `ADR-55` names, at the outermost layer -- the report's own verdict. Every
`RuleExecution` already carries `status` and `records_examined`; the summary simply does not consult
them.

## Why nothing caught it

Every test supplies a populated `rules` table, because a config without one was not a state anyone
intended to produce. `ADR-53` made rules opt-in, which made the empty table reachable, and nothing
revisited what an empty table should mean.

`ADR-55` was written the day before this was found and names exactly this signal as the one to
trust. It was not yet applied to the place the verdict is computed.

## References

- `ADR-55`, whose decision this is the parent case of, and `MILE-106`, which builds the rule form.
- `BUG-42`, which created `audit`'s path.
- `ADR-53`, which made an empty rules table reachable.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-05 | Realized-by locator `audit_with_neither_of_its_rules_declared_is_not_ok` re-pointed to `audit_with_no_rule_declared_is_not_ok`. **Why:** `MILE-190` renamed the test when `audit` stopped owning a rule set; it asserts the same behavior for a run in which no rule is declared. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#a_run_in_which_no_rule_examined_anything_is_not_ok` (lines 1847-1865) fbf927164325 -> cf759d07934a, `rust/crates/urzua-cli/tests/check_integration.rs#audit_with_no_rule_declared_is_not_ok` (lines 1961-1982) de68e88d0c24 -> de028eec3b26. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#a_run_in_which_no_rule_examined_anything_is_not_ok: the output assertions now read the JSON field with json_field instead of substring-matching the printed text, with the same expected values; check_integration.rs#audit_with_no_rule_declared_is_not_ok: the output assertions now read the JSON field with json_field instead of substring-matching the printed text, with the same expected values. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
