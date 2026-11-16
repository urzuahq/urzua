---
Stable-Id: 01M7FVXJX6H9XXRPGWQYM6E96A
Status: Planned
Phase: '1'
Track: code-health
Blocked-on: null
---
# 238 — Every rule reads typed options, and the flat rule setting is gone

## What

Finish the move to per-rule typed options: every rule reads its options through its own type, and the single struct of fifty optional fields is replaced by per-rule option types.

## Why

the audit of 2026-11-15 found typed options half adopted: fourteen implementations exist, ten sites still read the raw setting, and whether an option applies to a rule is checked at run time.

## Done means

- No rule reads `config.rules` directly, enforced by a test.
- An option set on a rule that does not take it is a load error produced from the rule's own option type.
- Adding an option to one rule changes that rule's type and the rule table only.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
