---
Stable-Id: 01M2VGDX8CRYPC8Y8V5Q5J14M4
Status: Fixed
Found-in: 'A code review of the unreleased diff since v0.3.0 -- reproduced for both directions, an inverted rule and an inert one'
Regression-test: 'rust/crates/urzua-core/src/config.rs::a_rule_missing_an_option_it_requires_is_rejected_observed_failing -- each required key rejected by name, `off` accepted without options, and a complete declaration still loading'
---
# 44 — A rule's required options are unvalidated, so an incomplete rule inverts or never fires

## What is wrong

`config::parse` validates that an option sits on the rule that owns it (`OptionNotApplicable`). It
never validates that a rule which **needs** an option was given one. Both directions are broken, and
they fail in opposite ways.

**Inverted.** `claim.status-agreement` without `closed_statuses` gets an empty set from
`unwrap_or_default()`, and the rule treats "not in the closed set" as a violation. Every claim becomes
a finding, including correct ones:

```
claims to close BUG-36, but BUG-36 has Status Fixed
```

A blocking error that contradicts itself.

**Inert.** `claim.status-agreement` without `claim_paths` scans nothing and reports `ran` forever.
`pointer.target-status` without `not_in` runs and can never fire. Both are the failure this project
names as the worse one -- and `BUG-40` is the report-side half of the same problem.

## Why it was missed

`MILE-80` added option validation in one direction only. The rules that take options were introduced
in the same change that declared them correctly in this repository's own config, so neither incomplete
shape was ever exercised.

## Fix

A rule declares which options it requires, and `parse` rejects a declaration missing one, naming the
key. This is the same shape as `BUG-41`'s companion-field pairs and `BUG-40`'s unreadable scope: a
check whose ability to run is silently conditional on configuration.

Worth building once, with `BUG-40` and `BUG-41`, rather than three times.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `MILE-80` validated that an option sits on the right rule and never that a required option is present. A rule missing one either inverts into an all-errors rule or becomes permanently inert, and both shipped unexercised because this repository declared them correctly from the start. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** `parse` now rejects a rule declared without an option it requires, naming the key and what the rule would otherwise do -- *"treats every claim as a violation, including correct ones"* for a missing `closed_statuses`, *"scans nothing and can never report"* for a missing `claim_paths`. `init` skips option-requiring rules rather than proposing a declaration that will not load, which `BUG-40`'s review had also flagged from the other side. | **substantive** |
> | 2026-09-19 | `off` no longer requires a rule's options. **Why:** review found that `pointer.target-status: off` was rejected for a missing `not_in` -- so the rule could not be turned off without supplying values it would never read, since `gated` skips a declined rule before any option is used. Turning a rule off is the one declaration that needs nothing. | **substantive** |
