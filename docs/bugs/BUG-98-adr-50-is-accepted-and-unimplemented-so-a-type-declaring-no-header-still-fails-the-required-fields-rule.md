---
Stable-Id: 01M30MEE250445FAWYQMRGNHX5
Status: Fixed
Found-in: "Auditing which rules could fire on a type declaring no header, while assigning populations to the rule set"
Regression-test: "a_type_declaring_no_header_is_not_reported_as_missing_one_observed_failing, a_type_declaring_no_header_is_not_reported_as_deprecated, a_header_none_type_with_required_fields_is_a_contradiction_observed_failing, rules.rs"
---
# 98 — ADR-50 is accepted and unimplemented so a type declaring no header still fails the required-fields rule

## What was wrong

`ADR-50` is `Status: Accepted` and decides that `header_shape` accepts `none`, meaning a type's
records carry no header block, and that `header.required-fields` skips such a type rather than
reporting a missing region.

`HeaderShape` had three variants — `Blockquote`, `BoldList`, `YamlFrontmatter`. There was no `none`.
A config declaring it failed to deserialize, so the decision could not be exercised at all, and a
corpus family whose records legitimately carry no header would have been reported as one missing a
header region on every record.

The accompanying config-level rule the ADR calls for — reporting `required_fields` declared non-empty
for a type that has nowhere to put a field — did not exist either.

## Why nothing caught it

Nothing checks that an `Accepted` ADR is implemented. `embodiment.consistency` would report this if
the ADR carried a `Realized-by` naming the code that implements it; it carried none, so the record
read as a decision in force while the engine had never behaved that way.

Because no adopted corpus in reach declares a header-less type, the gap produced no failing build and
nothing forced it into view.

## What changed

`HeaderShape::None` exists, deserializes as `"none"`, and `parse_with_shape` returns a header with no
region and no parse error for it — the declared absence, not a parse failure. `header.required-fields`
skips a record whose type declares `none`. A new rule, `config.header-none-has-no-required-fields`,
reports a `none`-shaped type declaring a non-empty `required_fields` as the self-contradiction `ADR-50`
calls it. `header.deprecated-shape`'s negative test (`!= YamlFrontmatter`) — which `ADR-50` warned
would immediately misreport `none` as deprecated — is now an explicit `match` over the two actually
deprecated shapes, exhaustive against a future fifth variant. `urzua new` refuses to generate a record
for a `none`-shaped type (nowhere to write an identity); `migrate ids` cannot reach that case at all,
since a `none`-shaped record's header always has no region, and the existing "no region" branch already
skips it before the shape-specific insertion code would run.

## References

- `ADR-50`, the decision this bug is the unimplemented half of.
- `ADR-55` — a decision that cannot fire is the same defect class as a rule that cannot fire.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-21 | Filed. **Why:** surfaced while assigning a population to every rule, which required knowing which types each rule can apply to. `header_shape: none` appears in a ratified decision and in no code path. Filed rather than implemented, because the ADR also calls for a config-level rule and the two belong in one change. | **substantive** |
> | 2026-09-22 | Fixed. **Why:** the user asked to fix this before the 0.4.0 release; implemented exactly what `ADR-50` already decided, with the consequences it named (`header.deprecated-shape`'s negative test, `urzua new`'s refusal) addressed in the same change rather than left as follow-up gaps. | **substantive** |
