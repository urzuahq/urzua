---
Stable-Id: 01M363T8C5DS4NNVS66QB6CSFJ
Status: Accepted
Date: 2026-09-23
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-43
---
# 62 — known fields declaration required is an opt-in rule not a schema mandate

## Context

`RFC-43` proposed a new opt-in rule, `config.known-fields-declaration-missing`, and left two questions
open: message wording, and whether this repository should enable the rule on its own config in the
same change.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| Message wording | Name the gap only vs. suggest the fix inline | `config.pointer-declaration-missing`'s own message already suggests the fix inline ("declare both explicitly, even as `[]`"); matching it keeps one voice across every `config.*-declaration-missing` rule |
| Enable here now | Enable and fix this repo's own gaps in the same PR vs. enable later, separately | `AGENTS.md`'s "use the tool on itself": a rule this repository doesn't dogfood on its own corpus is unverified in the one place its author can act on a finding immediately |

## Decision

In the context of `RFC-43`'s two open questions, **we decided to accept both in the more thorough
direction**: the finding message suggests the fix inline, matching `config.pointer-declaration
-missing`'s existing wording, and this repository enables `config.known-fields-declaration-missing`
in the same PR that ships the rule, declaring `known_fields` (even as `[]`) for every type that
currently omits it.

## Reversibility

Cheap. Disabling the rule is a one-line config removal; the declarations it prompts this repository to
add stay useful (or at worst inert) either way, since `known_fields: []` for a type with no field-set
governance intent so far means exactly what it says.

## Consequences

Every type in `.urzua/config.yaml` gains an explicit `known_fields` (or already has one), which also
turns on `header.field-set-consistency`'s strict enforcement for any type that previously relied on
the lenient unchecked default — a real, evidenced behavior change for this repository's own corpus,
verified by a real-corpus `check` diff before and after, not assumed safe.

## References

- `RFC-43` — the proposal this ADR decides.
- `config.pointer-declaration-missing` — the message-wording and rule-shape precedent.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Decided. **Why:** `RFC-43` left two open questions blocking implementation; both resolved toward dogfooding the rule immediately rather than shipping it inert. | **substantive** |
