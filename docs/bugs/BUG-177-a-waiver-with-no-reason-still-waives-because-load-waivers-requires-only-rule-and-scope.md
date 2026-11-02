---
Stable-Id: 01M6D5VMCJKG8X6YSHC5QZE8D4
Status: Open
Found-in: "A /code-review pass, round 30"
Regression-test: "planned: a waiver.rs test that a waiver lacking Reason is not active, once the intended behavior is decided"
---
# 177 — a waiver with no Reason still waives because load_waivers requires only Rule and Scope

## What was wrong

`load_waivers` skips a waiver only when `Rule` or `Scope` is missing. A waiver with no `Reason` still waives;
the omission is visible only when `header.required-fields` is enabled for the waiver type. `SPEC-10` lists
`Reason` as "free text, required", "the whole reviewability point of not using an ignore list".

## Why nothing caught it

`SPEC-10` describes the skip rule for `Rule`/`Scope` only; `Reason` is enforced only through the generic
required-fields rule, which is opt-in.

## Why this needs a decision, not a mechanical fix

Needs a decision: refuse to honor a reasonless waiver (fails toward more findings, like a missing `Rule`),
or keep enforcement in `header.required-fields` and say so in `SPEC-10`. Note, checked and not filed:
`Scope` matching exactly (so `./x`, a directory or a glob waives nothing) is explicit in `SPEC-10`'s
Matching semantics, and a typo'd `Rule` is listed there under "not built".

## References

`SPEC-10`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed, left `Status: Open`. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
