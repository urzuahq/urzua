---
Stable-Id: 01M2RTYR78PWE69G8116MWWPSH
Status: Open
Found-in: 'BUG-26 -- the `spec` type declared `Embodiment` without `Realized-by`, and nothing said so'
Regression-test: 'not yet written -- a type declaring one half of a companion pair and not the other must be a finding, for every pair, not only `pointer_fields`/`narrative_fields`'
---
# 41 — Companion-field pairs are checked for one pair and hardcoded to it

## What is wrong

`config.pointer-declaration-missing` already names this exact shape, in its own doc comment:

> a type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even
> as an empty array -- omitting one is not the same as deliberately declaring zero fields of that kind.

That is correct, and it is written for **one pair**. A second pair with identical semantics exists and
is unchecked: `Embodiment` and `Realized-by`.

`embodiment.consistency` reads both, and returns early if either is absent -- two `let ... else
{ continue; }` guards, before the examined counter increments. So a type declaring `Embodiment` in
`known_fields` but not `Realized-by` produces records that can claim an embodiment state **no rule is
able to check**. The `spec` type was configured that way, and nothing reported it.

## How it surfaced

Accepting three specs under `BUG-26`. `SPEC-1` was briefly written with `Embodiment: Verified` and no
`Realized-by` -- a false claim, of exactly the kind `embodiment.consistency` exists to catch, and
invisible to it by construction. It came to light only because the *other* two specs tripped
`header.field-set-consistency` on an undeclared field, which is a different rule noticing a different
thing.

That is `BUG-40`'s failure mode one level up: not a rule in scope of nothing, but a rule whose scope
the configuration can silently empty.

## Fix

Generalise, rather than adding a second hardcoded pair. Which fields a rule reads together is known
where the rule is defined; "these must be declared together" is a property of the pair, not of
`pointer_fields`. Under `ADR-53` the resulting check is a configured rule like any other.

Worth doing with `BUG-40` and not before it. Both are a check that cannot run failing to say so, and a
fix for one that ignores the other will be re-derived a third time. `RFC-33` already records that five
config-level rules share a byte-identical preamble, which is this same observation from the
implementation side.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-18 | Filed. **Why:** found while fixing `BUG-26`, and separated from that record because it is specific to neither specs nor `Embodiment`. The rule for this shape already exists and was written for one pair; a second pair with the same semantics went unchecked for as long as it has existed. | **substantive** |
> | 2026-09-19 | Deferred behind `MILE-98`. **Why:** fixable today as another hardcoded comparison in `rules.rs`, and that is the mistake this family *is* -- `PLACEHOLDER_TOKENS` transcribed by hand, `config.pointer-declaration-missing` hardcoded to one pair, `revision-log.change-class-required` keyed to a literal string. Each is a comparison written as a constant. Under the declared document model they are declarations, so building them now means building them twice and teaching the second version nothing. The gap stays open for the duration, deliberately. | **substantive** |
