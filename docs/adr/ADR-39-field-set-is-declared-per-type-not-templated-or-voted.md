---
Status: Accepted
Stable-Id: 01M1Z3HTHA9VHQX8WCXXX9TQBA
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/rules.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-core/src/rules.rs
Date: 2026-09-07
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: RFC-10 (Accepted)
---
# 39 — Field set is declared per type not templated or voted

## Context

Neither `header.required-fields` nor `header.layout-consistency` (ADR-38) catches a field present
on some records of a type but not others: SPEC-1 carries `Embodiment`/`Author`/`Derives-from` that
SPEC-2 through SPEC-6 don't, and nothing flagged it. `required_fields` only checks for a *missing*
required field, never for a field's mere *presence* being unexpected; `layout-consistency` checks
how existing fields are arranged, never which ones exist.

A naive fix -- treat `required_fields` as the complete, closed set for a type -- was checked
against the real corpus first and rejected immediately: ADRs and RFCs already carry fields
(`Stable-Id`, `Realized-by`, `Derives-from`, `Supersedes / Superseded-by`) that are legitimately
common but never marked required. Closing the set to `required_fields` alone would flag the entire
existing corpus as a false positive the moment the rule shipped.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| `required_fields` is the closed set, nothing else allowed | No new config | False-positives on every ADR/RFC's existing legitimate optional fields (`Stable-Id`, `Realized-by`, etc.) |
| Infer a type's "normal" fields from majority vote among existing records | No new config, works immediately | Ratifies whatever's already drifted in rather than catching it -- rejected for the same reason as `header.required-fields` and ADR-38's `header_layout` |
| Declare `known_fields` per type in `.urzua/config.toml`, additive to `required_fields` | Works without breaking existing optional fields; explicit, diffable; a type with none declared is simply unchecked | One more optional config key to document |

## Decision

In the context of a field's mere presence-or-absence across a type's records having no check at
all, facing a choice between inferring a type's allowed fields and declaring them, we decided:
**`known_fields` is an optional per-type config field, additive to `required_fields`, checked by a
new `header.field-set-consistency` rule.** A field not in `required_fields` ∪ `known_fields` for its
type is a `Warning` finding. A type with no declared `known_fields` is skipped entirely -- additive,
never a forced migration, and deliberately left undeclared for `spec` until MILE-74 decides its
canonical field set, rather than guessed at here.

This repo's own config now declares `known_fields` for every type with more than one settled shape:
`adr` (`Embodiment`, `Realized-by`, `Stable-Id`, `Derives-from`, `Supersedes / Superseded-by`),
`rfc` (`Supersedes / Superseded-by`, `Amends`), `milestone` (`Stable-Id`, `Implements`), `bug`
(`Realized-by`, `Stable-Id`), `waiver` (`Stable-Id`, `Expires`) -- `spec` intentionally excluded.

## Reversibility

Fully additive and cheap to reverse: `known_fields` is an optional config key and
`field-set-consistency` is one more rule in the existing list. Removing either leaves every other
rule and the existing schema untouched.

## Consequences

- A new rule, `header.field-set-consistency`, joins `check`'s rule set at `Warning` severity,
  consistent with how new rules have been introduced against a corpus not yet held to them.
- `RecordTypeConfig` gains `known_fields: Option<Vec<String>>`.
- MILE-81's question is answered: field-set drift gets a rule, declared per type, not inferred.
  Verified against a planted violation in an isolated scratch corpus, not only unit tests.
- `spec` stays unchecked by this rule until MILE-74 resolves its canonical field set -- SPEC-1's
  `Embodiment`/`Author`/`Derives-from` remain visible drift, tracked, not silently accepted.

## References

- RFC-10 — the closed-header model this decision extends to a new axis (which fields exist, not
  how they're arranged).
- ADR-38 — the header-layout-consistency precedent this decision follows exactly, on the field-set
  axis instead of the layout axis.
- MILE-74 — the milestone that must resolve before `spec` can get a `known_fields` declaration.
- MILE-81 — the milestone this ADR resolves.
