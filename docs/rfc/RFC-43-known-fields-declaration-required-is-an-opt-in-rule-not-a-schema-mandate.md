---
Stable-Id: 01M363DVQVHNSP0H8YVKQ1HJ6H
Status: Accepted
Date: 2026-09-23
Author: beauwilliams
---
# 43 — known fields declaration required is an opt-in rule not a schema mandate

## Summary

A record type that never declares `known_fields` gets no field-set governance at all today —
`header.field-set-consistency` skips it entirely, so any field, including a case-variant of a field
the type already governs elsewhere (e.g. a lowercase `status:` alongside a declared `Status`), passes
silently. This proposes a new opt-in rule, `config.known-fields-declaration-missing`, mirroring
`config.pointer-declaration-missing`: a repository can require every type to make the choice
explicit, without mandating it for every adopter as a schema change.

## Motivation

A round-20 code review flagged this exact gap: a type with no `known_fields` never has an
undeclared-field case-mismatch caught anywhere, since `header.field-case-mismatch` (`ADR-58`) and
`header.field-set-consistency` (`ADR-39`) both only examine declared slots — `ADR-53`'s "declared, not
voted" principle, working as designed. That review's finding was correctly refuted as intentional
behavior, not a bug — but the underlying question it raised is real: should a repository be able to
*require* every type to have made a conscious field-set decision, the same way `config.pointer
-declaration-missing` already requires `pointer_fields`/`narrative_fields` to be declared together
rather than left ambiguous by omission?

`ADR-53` argues declared-not-voted is correct as the *default* — an adopter migrating a type
incrementally shouldn't be blocked by a check for governance it hasn't gotten to yet. But "the
default is lenient" and "there is no way to opt into strictness" are different claims, and this
repository's own `config.pointer-declaration-missing` already proves the pattern for one axis.

## Proposal

A new rule, `config.known-fields-declaration-missing`: for each configured record type, if
`known_fields` is `None` (never declared, as distinct from `Some(vec![])`, declared empty), report a
finding naming the type. Off by default like every other rule (`ADR-53`) — a repository opts in by
setting it to `warn`/`error` in `.urzua/config.yaml`, the same declared choice `config.pointer
-declaration-missing` already is.

This is additive only:
- `header.field-set-consistency`'s own behavior is unchanged. Declaring `known_fields: []` already
  makes it enforce strictly (any field beyond `required_fields` is flagged) — that mechanism exists
  today and needs no change; this RFC only proposes a way to *require* the declaration be made.
- No existing config's behavior changes unless it explicitly enables the new rule.
- Implementation is a near-duplicate of `config_pointer_declaration_missing`'s existing shape: a
  `RecordType`-population rule, one condition (`known_fields.is_none()`), one `Finding`.

## Open questions

- **Message wording**: should the finding suggest the fix inline (`"declare known_fields, even as
  [], to make this type's field-set governance an explicit choice"`) or just name the gap? Leaning
  toward the former, matching `config.pointer-declaration-missing`'s own message style.
- **Should this repository enable it on its own config in the same change?** This repo has several
  types (check via `.urzua/config.yaml`) that currently omit `known_fields`; enabling the rule here
  would need each of those types to make the choice explicitly as part of landing this RFC, not left
  for a later pass.
- **Naming**: `config.known-fields-declaration-missing` vs. a more general name if a future review
  finds the same "no way to require this declaration" gap for another `Option`-typed config field
  (e.g. `header_layout`). Proposing the specific name now; a general mechanism is not evidenced yet
  (`AGENTS.md`'s "don't build speculative capability").

## Non-goals

- Does not make `known_fields` mandatory by default, or introduce a schema-version bump — this is an
  ordinary new rule, like every other rule in `ALL_RULES`.
- Does not change what "declared empty" (`known_fields: []`) means or how strictly it's enforced —
  that behavior is unchanged and already shipped.
- Does not attempt to catch a case-variant field when `known_fields` stays undeclared and this new
  rule stays off — that combination is `ADR-53`'s accepted lenient default, unchanged by this RFC.

## References

- `ADR-53` — "declared, not voted"; the principle this RFC extends with an opt-in strictness lever
  rather than reopening.
- `ADR-39` — `header.field-set-consistency`'s own declared-only scope, unchanged by this proposal.
- `config.pointer-declaration-missing` (`MILE-90`/`ADR-44`) — the existing rule this RFC's shape
  mirrors exactly.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed as `Draft`. **Why:** a round-20 code review's refuted finding (an undeclared-field case-mismatch going unchecked) raised a real question underneath the refutation — the user asked for an opt-in way to require the declaration, not a mandate, matching `config.pointer-declaration-missing`'s existing pattern. | **substantive** |
> | 2026-09-23 | Accepted via `ADR-62`: inline fix-suggesting message, and this repository enables the rule on its own config in the same change. | **substantive** |
