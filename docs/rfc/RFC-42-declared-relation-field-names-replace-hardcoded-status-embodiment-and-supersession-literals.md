---
Stable-Id: 01M35PK7FGGHVAYRTHJD6SDKW0
Status: Accepted
Date: 2026-09-23
Author: beauwilliams
---
# 42 — Declared relation field names replace hardcoded status embodiment and supersession literals

## Summary

Six rule functions read a target record's lifecycle or relation field by a field-name string literal
baked into the rule body (`"Status"`, `"Embodiment"`, `"Realized-by"`, `"Supersedes / Superseded-by"`)
instead of an adopter-declared name, unlike `pointer_fields`/`narrative_fields`, which are already
declared config keys (`MILE-90`). This proposes one general mechanism — a per-type map from a fixed
set of *roles* (`status`, `embodiment_state`, `embodiment_locator`, `supersession`) to adopter-chosen
field names, each defaulting to today's literal so no existing config needs to change — replacing all
four hardcoded literals with a single declared-field lookup, the same shape `pointer_fields` already
uses.

## Motivation

`BUG-59` established the precedent for this exact class of gap: a hardcoded per-record-type table of
terminal status *values* went permanently inert for a corpus naming its statuses differently, and the
fix was to make the vocabulary declared rather than compiled in. `BUG-110` found the same failure one
level up — `claim_status_agreement`, `pointer_target_status`, and `narrative_field_stale` all read a
target's lifecycle field via the literal `"Status"`. A round-18 code review then found the identical
shape recurring in four more places:

- `embodiment_consistency` and `embodiment_locator_promotion_candidate` read `"Embodiment"` and
  `"Realized-by"` (`rules.rs:2405`, `2409`, `2472`)
- `embodiment_locator_exists` reads `"Realized-by"` (`rules.rs:2326`)
- `supersession_reciprocity` reads `"Supersedes / Superseded-by"` (`rules.rs:2603`, `2656`)

That is six rules across three relation families sharing one defect shape: an adopter whose corpus
calls these fields anything else gets silent non-detection, indistinguishable from a clean corpus —
exactly the failure mode `ADR-55` exists to prevent, now hiding behind a literal rather than a missing
gate. `ADR-60` fixed the adjacent, narrower question for `Status` (gate the read on the target type
declaring the field) without addressing the field name itself; this proposal is the general form that
question turned out to have.

Filing one RFC for all four literals, rather than four separate `BUG-110`-shaped patches, because the
same schema decision (a role → field-name map, defaulted, threaded through the same
`declared_cross_record_value` helper `ADR-60` already built) resolves all of them identically; patching
them independently would produce four slightly different mechanisms for the same problem.

## Proposal

Add one optional per-type config key, `relation_fields`, mapping a fixed set of role names to the
field name that plays that role for this type:

```yaml
record_types:
  bug:
    relation_fields:
      status: Status          # default if omitted
  adr:
    relation_fields:
      status: Status
      embodiment_state: Embodiment
      embodiment_locator: Realized-by
  rfc:
    relation_fields:
      supersession: "Supersedes / Superseded-by"
```

The role set is fixed (`status`, `embodiment_state`, `embodiment_locator`, `supersession`) — this is
not an arbitrary-alias mechanism like `pointer_fields`, because each role has role-specific behavior
(`status` feeds `field.pending`'s terminal-status set from `BUG-59`; `embodiment_state`/
`embodiment_locator` are read as a pair by `ADR-60`'s gating). Each role defaults to its current
literal when the type doesn't declare `relation_fields` or omits that role, so every existing config
keeps behaving exactly as today — this is additive, not a schema-version bump.

`RecordTypeConfig` gains `pub relation_fields: Option<RelationFields>` (a small struct with four
`Option<String>` fields, `#[serde(deny_unknown_fields)]` per the project's existing convention), and a
`fn relation_field(&self, role: RelationRole) -> &str` accessor returning the declared name or the
role's default. The six call sites (`claim_status_agreement`, `pointer_target_status`,
`narrative_field_stale`, `embodiment_consistency`, `embodiment_locator_exists`,
`embodiment_locator_promotion_candidate`, `supersession_reciprocity` — seven, not six; see Open
questions) replace their literal argument to `declared_value`/`declared_cross_record_value` with
`type_config.relation_field(RelationRole::Status)` etc. `declared_cross_record_value`'s own
declaration-gating (`ADR-60`) is unchanged — it already takes the key as a parameter, so this is a
caller-side change only.

A record whose type declares a non-default name for a role that another rule still reads under the
default is exactly the divergence this RFC exists to prevent: `field.quality`/`field.pending`
generically iterate `required_fields`/`known_fields` and already respect whatever name is declared
there, so no additional wiring is needed beyond the six rule bodies above and their tests.

## Open questions

- **Is `supersession_reciprocity` in scope?** `rules.rs:2603` declares `FIELD` as a local `const`
  already, not an inline repeated literal — arguably better-factored than the other three already.
  Bringing it under `relation_fields` is still worth doing for consistency (one config surface, one
  accessor) but could be split into a smaller follow-up PR if reviewers want the`status`/`embodiment`
  half landed first.
- **Should `relation_fields` be validated against `known_fields`/`required_fields`** the way
  `pointer_fields`/`narrative_fields` are (`config.pointer-narrative-overlap`)? Likely yes — a
  declared `relation_fields.status: State` that isn't also in `required_fields`/`known_fields` would
  silently never be examined under `ADR-60`'s gate, the same trap `BUG-109` closed for the unqualified
  case. Proposed: a new `config.relation-field-not-known` rule, mirroring
  `config.pointer-field-not-known`.
- **Naming**: `relation_fields` vs. something narrower per family (`status_field` +
  `embodiment_fields` + `supersession_field` as three separate keys instead of one map). The single
  map keeps the config surface small and the accessor uniform; three keys would read slightly more
  discoverably in a config file. Proposing the single map; open to the alternative if reviewers prefer
  it.

## Non-goals

- Does not touch `pointer_fields`/`narrative_fields` themselves — those are already declared.
- Does not introduce case-insensitive or fuzzy field-name matching — `ADR-57`'s exact-comparison rule
  is unaffected; this only changes *which* literal is compared, not how.
- Does not change `ADR-60`'s declaration-gating semantics, only what key is looked up.
- Does not attempt a fully generic "any field can play any role" mechanism — the role set is fixed and
  small, matching what the engine's rule bodies actually need today.

## References

- `BUG-59` — the precedent: a hardcoded per-type status-value table went inert for a differently-named
  corpus; the fix made the vocabulary declared.
- `BUG-110` — the narrower, `Status`-only version of this finding, superseded by this RFC's broader
  scope.
- `ADR-60` — decided the adjacent question (gate the cross-record `Status` read on declaration)
  without deciding this one (make the field name itself declared); this RFC's accessor is built on
  top of `ADR-60`'s `declared_cross_record_value`, unchanged.
- `MILE-90`/`ADR-44` — made pointer/narrative field names declared instead of a hardcoded
  `"Blocked-on"`, the precedent this RFC generalizes to the remaining hardcoded relation fields.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed as `Draft`. **Why:** a round-18 code review found the same hardcoded-field-name defect shape (`BUG-110`) recurring in four more rules; consolidating into one design decision rather than patching each independently. | **substantive** |
> | 2026-09-23 | Accepted via `ADR-61`, all three open questions resolved in favor of the broadest consistent scope. | **substantive** |
