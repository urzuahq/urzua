---
Stable-Id: 01M6HZ0NMJDR3KN60CTNJ4CQ6P
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
---
# 140 — Per-field severity within one rule

## What

Severity is set per field within one rule, not only per rule. A rule such as `field.quality` can raise a blank value in one field to an error and the same state in another field to a warning, from configuration.

## Why

`MILE-80` made severity configurable per rule, so one rule has one level for every field it covers. The adopter blocks on a blank Deciders on an accepted record but only warns on a blank Reviewers, and those two cannot coexist while `field.quality` has a single level. Splitting the rule into one rule per field is the alternative; it multiplies rule ids for what is one check, so it is worth deciding explicitly which of the two to build.

## Done means

A config can give two fields of one rule different severities, a finding carries the severity of its own field, an unlisted field takes the rule-level severity, and the choice between per-field severity and splitting the rule is recorded.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the adopter's linter blocks on one blank role field and warns on another, which urzua's per-rule severity (`MILE-80`) cannot express. | **substantive** |
