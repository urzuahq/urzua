---
Stable-Id: 01M7V84TDVKYBVWKZV81EBNG76
Status: InProgress
Phase: '1'
Track: embodiment-model
Implements: ADR-74
Blocked-on:
---
# 352 — A symbol locator tells a method from a field of the same name, so line-range pins can be retired

## What

A `path#symbol` locator names a method and a field of the same name apart (for example `Type::method` against the field `method`), so a claim about either can be pinned to its own span. Today the resolver cannot tell them apart, and the only precise pin left is a line range.

## Why

`Population::unreadable` is both a field and a method of `Population`, and `path#symbol` cannot name one without the other. `MILE-111` (`report.rs#L743-L748`) and `MILE-89` (`config.rs#L336-L376`) therefore pin a line range, which goes stale whenever lines above it move; one of them went stale while an unrelated pull request (`#609`) was in review. A line-range pin is a whole-file pin in disguise for churn, which `MILE-284` set out to reduce. `ADR-74` decides the scoped-locator grammar; a way to name a member of an impl against a field may need that grammar extended by a decision of its own, and that decision is part of this work.

## Done means

- A locator names a method and a field of one name apart, with a documented spelling, and `urzua fix --pin` resolves each to its own span. — evidence: test:rust/crates/urzua-core/src/locator/tests.rs#a_field_and_a_method_of_one_name_each_resolve_to_their_own_span_by_kind@sha256:5e1fd3c88e385d5f5e1918a79b5fcb3fffa09397b69cf3a108010b36550e24f9, test:rust/crates/urzua-core/src/locator/tests.rs#a_kind_suffix_parses_on_the_last_member_segment_before_the_pin@sha256:20e841ae121ebf7d6bba6800ff2c01e291507d3991782ecd5d8fe86b273c7291, code:rust/crates/urzua-core/src/locator.rs#parse_symbol_scope@sha256:f5230b4fedb3bed64574df85d3e7fd316bde87dd8bef088199842f4a549f85c2, code:rust/crates/urzua-core/src/locator.rs#Resolution::member_in@sha256:0b2ded213ec3fb9537927d88f6618eedccd2efaaef6bfaf5a1f19279b4dc6173, spec:docs/specs/SPEC-3-configuration.md
- `MILE-111` and `MILE-89` pin the symbol they cite instead of a `#Lx-Ly` range, and no record in `docs/` pins a line range for want of a symbol that names the span.
- The resolver is exercised in this repository's own `.urzua/config.yaml` (`locator_resolvers`) and by a corpus case, not only by unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Filed as planned work: nothing built, no evidence yet. | **structural** |
> | 2026-11-22 | Partly delivered. The spelling is `Type::name[kind]`, an optional `kind` on each resolver member entry selecting the entries of that kind (`ADR-74` amendment of 2026-11-22, `SPEC-3` vectors 25-29), with unit tests and the `corpus-ao` acceptance cases, so item 1 names its evidence. `MILE-89` and one `MILE-198` range now pin symbols. What remains: item 3 needs the `kind: fn` / `kind: field` entries in this repository's `.urzua/config.yaml`, a protected path the maintainer edits; item 2 needs that too for `MILE-111` (`report.rs#Population::unreadable[fn]`), and the ranges in `MILE-191`, `MILE-198` and `MILE-284` that no symbol resolves today are `MILE-357`. **Why:** the maintainer chose the declared kind suffix (2026-11-22, AskUserQuestion answer to the orchestrating agent): question "MILE-352: ... How should a locator tell them apart? Any grammar change is an ADR-74 amendment you'd accept.", answer "Declared kind suffix (Recommended)". | **structural** |
> | 2026-11-22 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 0 file-level, 4 scoped. Evidence: rust/crates/urzua-core/src/locator/tests.rs#a_field_and_a_method_of_one_name_each_resolve_to_their_own_span_by_kind@sha256:5e1fd3c88e38, rust/crates/urzua-core/src/locator/tests.rs#a_kind_suffix_parses_on_the_last_member_segment_before_the_pin@sha256:20e841ae121e, rust/crates/urzua-core/src/locator.rs#parse_symbol_scope@sha256:f5230b4fedb3, rust/crates/urzua-core/src/locator.rs#Resolution::member_in@sha256:0b2ded213ec3 Reason: item 1: the unit tests show a field and a method of one name resolving to their own spans by kind and the suffix parsing, parse_symbol_scope is the grammar and Resolution::member_in the kind selection, SPEC-3 documents the spelling **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-22 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (1 of 3 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
