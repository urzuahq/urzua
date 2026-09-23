---
Stable-Id: 01M376A4R1WWWNH1SG401X1RHH
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 26"
Regression-test: "not yet written -- Status: Open, no fix decided yet"
---
# 145 — two hand-maintained rule-id allowlists carry the same drift risk RuleOption was built to eliminate

## What was wrong

`RULES_REPORTING_OUTSIDE_THE_CORPUS` (consulted by `check.rs`, exempting a finding from path-scope
filtering) and `IDENTITY_DEPENDENT_RULES` (consulted by `init.rs`, deciding which rules adopt-mode
skips for a prefixless corpus) are both hand-maintained `&[&str]` allowlists of rule ids, each own doc
comment citing a past bug (`BUG-61`/`67`/`86`/`70`) caused by exactly this "a per-rule fact lives in a
list nothing forces a new rule to be added to" shape.

`RuleOption` (`config.rs`) was introduced specifically to centralize per-rule facts of this kind -- its
own doc comment: "a hand-maintained list per fact would let drift apart." These two lists are the same
shape of fact RuleOption was built to stop hand-maintaining, just not folded into it.

## Why this needs a decision, not a mechanical fix

`RuleOption`'s current schema only tracks config-option ownership (`name`, `owner`,
`consequence_if_missing`) -- it has no room for "reports outside the corpus" or "derives identity from
a filename prefix" as declared facts. Unifying these would mean extending `RuleOption`'s own fields (or
introducing a second, differently-shaped per-rule fact table), a real schema decision about what belongs
in one place versus staying as its own constant, not a drop-in reuse of the existing type.

## References

- `RuleOption` (`config.rs`) -- the existing per-rule-fact table this bug's fix would either extend or
  sit beside.
- `BUG-61`, `BUG-67`, `BUG-86`, `BUG-70` -- the past incidents each allowlist's own doc comment already
  cites as the reason it exists.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** found by a full-release code review; verified both are genuinely hand-maintained rule-id lists with no mechanism forcing a new rule to be added to either. Left `Status: Open` -- unifying them into `RuleOption` needs deciding that type's schema, not a quick edit. | **substantive** |
