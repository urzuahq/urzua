---
Stable-Id: 01M6D57BDWZY0HRJABVQB8FBJ9
Status: Fixed
Found-in: "Reviewing why ADR-48 and ADR-49 warned on a Derives-from pointing at a Superseded ADR"
Regression-test: "check_integration.rs's target_status_fields_restricts_which_declared_fields_warn_observed_failing test, which asserts that with fields: [Implements] only the Implements slot warns, and target_status_without_fields_warns_on_every_declared_field, which asserts the omitted option is unchanged"
---
# 172 — pointer.target-status cannot distinguish lineage fields from dependency fields

## What was wrong

`pointer.target-status` applied one `not_in` list to every declared pointer and narrative field. The
fields mean different things: `Implements` and `Blocked-on` name a current dependency, so a Superseded
target is stale, while `Derives-from` names lineage, where pointing at a Superseded record is
permanently true history. On this repository `ADR-48` and `ADR-49` (`Derives-from: ADR-29`, which is
Superseded) warned falsely. The only remedies were rewriting Accepted ADRs, which the project forbids,
or waiving a finding that was never a defect.

## Why nothing caught it

The rule was written when `not_in` was the only knob, and every declared field was assumed to carry
the same obligation. `BUG-8` made the field list config-declared, but gave the rule no way to treat
two declared fields differently.

## Fix

Added an optional `fields` rule option on `pointer.target-status`, a list of declared
pointer/narrative field names to check. Omitted, the rule behaves as before. Fields left out are
dropped from the rule's population rather than counted: `OutOfScope` means "fix the configuration" and
trips `config.scope-matches-nothing`, and `Absent` would claim the record left the field unwritten,
while this exclusion is the configuration working as written. A `fields`
entry naming no declared pointer or narrative field of any type fails config load, like any other
misdeclared option, so a typo cannot silently exclude nothing. This repository's config now lists every
declared field except `Derives-from`.

## References

- `BUG-8` -- the genericity fix this extends: the field list is declared, and now so is which declared
  fields this rule judges.
- `SPEC-2` -- the rule's option is documented in its rule table.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed and fixed in one pass. **Why:** a single `not_in` list could not express that a Superseded lineage target is history while a Superseded dependency target is stale. | **substantive** |
