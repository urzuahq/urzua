---
Stable-Id: 01M2RTYR78PWE69G8116MWWPSH
Status: Fixed
Found-in: 'BUG-26 -- the `spec` type declared `Embodiment` without `Realized-by`, and nothing said so'
Realized-by: test:rust/crates/urzua-core/src/rules/companion.rs#a_type_declaring_one_half_of_a_declared_pair_is_reported_observed_failing@sha256:8c8d2528633801b49d66113f022d8840268ad34c694a3ffdfaf9ca567b8abf84, test:rust/crates/urzua-core/src/rules/companion.rs#the_pair_is_whatever_the_roles_name_not_embodiment_observed_failing@sha256:f5e0aa59adde5e3bbe4565d247d6beed11c7fbf587292b014c89a73039a66fa0, test:rust/crates/urzua-core/src/config.rs#a_companion_pair_naming_no_relation_role_is_refused_observed_failing@sha256:1a88a0d25662909e1ca131cb01016d34cdd9dc47e7e11ab8f397fc6e4afbf721
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
> | 2026-09-19 | `Blocked-on: MILE-98` declared as a field rather than described in prose. **Why:** the `bug` type did not declare `Blocked-on`, so the deferral was written into this log where no rule could see it. That is a configuration gap, not a missing rule -- `ADR-53` makes the field set a repository's declaration, and declaring it took one line. `narrative-field.stale` now reports all six of these when `MILE-98` reaches a terminal status. | **structural** |
> | 2026-11-04 | Relation pairs (`MILE-122`) are declared in config and checked by `relation.pair-reciprocity`; a pair field no type declares fails the load. The `Embodiment` / `Realized-by` companion check this record describes is not part of that and is still open, so `Status` stands. **Why:** `MILE-122` generalises the reciprocity half of this gap only. | **substantive** |
> | 2026-11-04 | `Blocked-on` cleared (`MILE-98` is Done); `Status` stays Open. Re-checked on main: `config.pointer-declaration-missing` still covers one pair, and a type declaring `Embodiment` without `Realized-by` (the roles `embodiment.consistency` reads together) is still unreported. **Why:** the declared document model does not change this fix, which the record already says is a generalisation over the pairs a rule reads together; it was held only so as not to write a second hardcoded pair, and `MILE-122` has since generalised the reciprocity half. It is buildable as its own change and is left for one, not folded into the document-model work. | **substantive** |
> | 2026-11-04 | Fixed. `companion_fields` declares pairs of relation roles per type (`SPEC-3`) and `config.companion-declaration-missing` reports a type declaring one field of a pair without the other, or neither; `adr` and `spec` declare the `Embodiment` / `Realized-by` pair in this repository's config. `config.pointer-declaration-missing` is unchanged: it checks declaration keys rather than header fields, and generalising it would need a built-in default pair. **Why:** `MILE-98` landed, so the pair is a declaration and no longer another hardcoded comparison. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 3 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
