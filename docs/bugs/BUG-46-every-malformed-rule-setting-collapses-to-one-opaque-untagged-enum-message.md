---
Stable-Id: 01M2VH55003E2WS281C12H8BAQ
Status: Fixed
Found-in: 'A code review of the unreleased diff since v0.3.0 -- reproduced for all three malformed shapes'
Regression-test: 'rust/crates/urzua-core/src/config.rs::a_malformed_rule_setting_names_what_was_wrong_observed_failing -- a bad level bare, a bad level in table form, and an unknown key, each asserting its own message and that none mentions `untagged`'
---
# 46 — Every malformed rule setting collapses to one opaque untagged-enum message

## What is wrong

`RuleSetting` deserializes through `#[serde(untagged)] enum Either { Bare, Table }`, and untagged
discards the inner variants' errors. Every malformed shape produces the same message:

```
could not parse config: rules: data did not match any variant of untagged enum Either at line 7 column 3
```

Reproduced for all three: `field.quality: eror`, `{level: eror}`, and `{levl: error}`.

So the hand-written message at `config.rs:64` --

> unrecognized rule level 'eror' -- expected "off", "warn", or "error"

-- is **dead code**, and so is `deny_unknown_fields`'s key name on the table form. `ADR-53` states that
an unknown key must be *"a load-time error naming the valid set"*, and `MILE-80` shipped a rule that
names nothing.

## Why it matters

This project is agent-native, and `ADR-53` records the specific reason: `OPA`'s maintainers found that
26% of Rego users learn the language from an LLM, and that a wrong guess must fail loudly rather than
be skipped. It fails loudly here and says nothing useful, which is half of what was decided.

## Fix

Dispatch manually on the parsed `Value` -- a scalar takes the bare path, a mapping the table path --
so each branch's own error survives. `untagged` is the convenience that costs the diagnostics, and the
diagnostics are the point.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** review of the unreleased diff. `untagged` discards inner variant errors, so the message `MILE-80` wrote to name the valid levels is unreachable, and `ADR-53`'s requirement that a wrong key names its alternatives is unmet by the change that claimed to implement it. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** `RuleSetting` dispatches on the parsed value -- a mapping takes the table path, a scalar the bare one -- so each branch's error survives. `untagged` was discarding them, which made the message naming the valid levels unreachable and left `ADR-53`'s requirement that a wrong key name its alternatives unmet by the change that claimed to implement it. | **substantive** |
