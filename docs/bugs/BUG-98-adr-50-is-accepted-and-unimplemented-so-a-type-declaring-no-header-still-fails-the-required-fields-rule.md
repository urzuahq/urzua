---
Stable-Id: 01M30MEE250445FAWYQMRGNHX5
Status: Open
Found-in: "Auditing which rules could fire on a type declaring no header, while assigning populations to the rule set"
Regression-test: "not yet written -- a type declaring `header_shape: none` must not be reported as missing a header region"
---
# 98 — ADR-50 is accepted and unimplemented so a type declaring no header still fails the required-fields rule

## What was wrong

`ADR-50` is `Status: Accepted` and decides that `header_shape` accepts `none`, meaning a type's
records carry no header block, and that `header.required-fields` skips such a type rather than
reporting a missing region.

`HeaderShape` has three variants — `Blockquote`, `BoldList`, `YamlFrontmatter`. There is no `none`.
A config declaring it fails to deserialize, so the decision cannot be exercised at all, and a corpus
family whose records legitimately carry no header is reported as one missing a header region on every
record.

The accompanying config-level rule the ADR calls for — reporting `required_fields` declared non-empty
for a type that has nowhere to put a field — does not exist either.

## Why nothing caught it

Nothing checks that an `Accepted` ADR is implemented. `embodiment.consistency` would report this if
the ADR carried a `Realized-by` naming the code that implements it; it carries none, so the record
reads as a decision in force while the engine has never behaved that way. That is a governance gap of
the same shape as the ones this engine exists to catch, and it is the argument for `MILE-106`-style
coverage over the decision corpus itself rather than only over the rules.

Because no adopted corpus in reach declares a header-less type, the gap produced no failing build and
nothing forced it into view.

## References

- `ADR-50`, the decision.
- `ADR-55` -- a decision that cannot fire is the same defect class as a rule that cannot fire.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Filed. **Why:** surfaced while assigning a population to every rule, which required knowing which types each rule can apply to. `header_shape: none` appears in a ratified decision and in no code path. Filed rather than implemented, because the ADR also calls for a config-level rule and the two belong in one change. | **substantive** |
