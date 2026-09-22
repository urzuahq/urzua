---
Stable-Id: 01M35PMJ7Z615RVYZ6TD9GSJ3D
Status: Accepted
Date: 2026-09-23
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-42
---
# 61 — declared relation field names replace hardcoded status embodiment and supersession literals

## Context

`RFC-42` proposed replacing seven rule bodies' hardcoded field-name literals (`"Status"`, `"Embodiment"`,
`"Realized-by"`, `"Supersedes / Superseded-by"`) with a per-type declared `relation_fields` map,
defaulted to today's literals, and left three questions open: whether `supersession_reciprocity` is in
scope, whether a declared-but-not-`known_fields` name should get its own validation rule, and whether
the config surface should be one map or several separate keys.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| Scope | All four field families in one RFC/ADR vs. splitting `status`/`embodiment` from `supersession` | One PR still touches every call site; splitting doubles the changeset/RFC/ADR overhead for a mechanism that's identical either way |
| Validation | Add `config.relation-field-not-known`, mirroring `config.pointer-field-not-known` vs. leave undeclared-but-configured names to fail silently under `ADR-60`'s existing gate | `BUG-109` was exactly this gap for the unqualified case; leaving it open here reintroduces the same defect one config key later |
| Config shape | One `relation_fields` map with four fixed roles vs. three separate top-level keys (`status_field`, `embodiment_fields`, `supersession_field`) | A single map keeps `RecordTypeConfig`'s field list from growing by three, and one accessor (`relation_field(role)`) serves every call site uniformly; the roles are already fixed and small, so the map's role names carry the same discoverability a separate key would |

## Decision

In the context of seven rules reading four field-name literals with no adopter-declared override, facing
open questions about scope, validation, and config shape, **we decided to accept `RFC-42` in full**:

- **Scope**: all four literals (`Status`, `Embodiment`, `Realized-by`,
  `Supersedes / Superseded-by`) move under `relation_fields` in one implementation round, including
  `supersession_reciprocity` — its `FIELD` constant is already well-factored, but a second lookup
  mechanism next to `relation_fields` for one field would itself be the kind of inconsistency `ADR-53`
  argues against.
- **Validation**: a new rule, `config.relation-field-not-known`, reports a type that declares a
  `relation_fields` entry not also present in that type's `required_fields`/`known_fields` — the same
  shape as `config.pointer-field-not-known`, closing the gap `BUG-109` closed for the unqualified case
  before it can reopen here.
- **Config shape**: a single `relation_fields: Option<RelationFields>` map with four fixed
  `Option<String>` roles (`status`, `embodiment_state`, `embodiment_locator`, `supersession`), each
  defaulting to its current literal when unset — chosen over three separate keys because the roles are
  fixed by what the engine's rule bodies need, not adopter-extensible like `pointer_fields`, so one
  accessor and one config surface is simpler without losing discoverability.

Every role defaults to today's literal, so no existing config's behavior changes until it opts into a
different name — additive, not a schema-version bump.

## Reversibility

Cheap to reverse for any single role (drop the accessor call, restore the literal at that call site).
Reversing the whole mechanism means restoring the literal at each of the seven call sites and deleting
`config.relation-field-not-known` and its tests — mechanical, no data migration, since `relation_fields`
is optional and additive.

## Consequences

`RecordTypeConfig` gains one new optional field. Seven call sites (`claim_status_agreement`,
`pointer_target_status`, `narrative_field_stale`, `embodiment_consistency`, `embodiment_locator_exists`,
`embodiment_locator_promotion_candidate`, `supersession_reciprocity`) read their field name through
`relation_field(role)` instead of a literal. A new config rule joins `ALL_RULES`. `BUG-110` is closed
by this ADR's implementation rather than fixed independently — its narrower `status_field`-only framing
is superseded by `RFC-42`'s general mechanism.

## References

- `RFC-42` — the proposal this ADR decides.
- `BUG-59`, `BUG-110` — the precedent and the narrower instance this generalizes.
- `ADR-60` — the declaration-gating mechanism this ADR's accessor is built on top of, unchanged.
- `MILE-90`/`ADR-44` — the `pointer_fields`/`narrative_fields` precedent this ADR extends to the
  remaining hardcoded relation fields.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Decided. **Why:** `RFC-42` named three open questions blocking implementation; resolved all three in favor of the broadest consistent scope (all four literals, one config surface, with its own validation rule) rather than a narrower patch that would leave the same gap one field later. | **substantive** |
