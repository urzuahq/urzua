---
Stable-Id: 01M6M7237A9CBQAHZJKYEK5ET7
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-8, ADR-64
---
# 73 — A finding carries a structured suggested_action with a closed set of kinds

## Context

`MILE-44` asks for a `suggested_action` on every finding. `RFC-8` and `ADR-15` name the field (`suggested_action`) on `urzua fix` detect entries and on Tier 3 findings that cannot be repaired unambiguously, and `MILE-44`'s Why says `SPEC-2` already documents `suggestedAction`. It does not: `BUG-21` rewrote `SPEC-2`'s output contract to the real shipped shape, and `Finding` (`report.rs`) has `rule`, `severity`, `file`, `line`, `message`, `waived` and nothing else. No accepted record defines the field's shape, which means the first implementation would define it by accident.

The shape matters because it is an output contract with 53 rules behind it and agents parsing it. `ADR-64` settles one half: output an engine expects a consumer to act on is structured, not prose. A prose string ("set the field to a real value") would be the escape hatch `ADR-64` closes. What is not settled is the structure: which kinds of action exist, what each carries, and what the engine may claim to know.

The rules fall into groups that need different actions (53 rules at 2026-11-04):

- A computed correction exists and `urzua fix` can write it: `embodiment.consistency`, `status.delivery-agreement`, `index.freshness` when regenerable.
- A header field needs a value only a human can supply: `field.quality` blank or placeholder, `header.required-fields`, `status.required-field-missing`.
- A header field must be removed or renamed: `status.forbidden-field-present`, `header.field-case-mismatch`, `header.case-variant-key`, `field.untrimmed-value`.
- A relationship needs its counterpart: `relation.pair-reciprocity`, `relation.supersession-reciprocity`.
- A configuration edit: the `config.*` rules and `type.*` rules.
- A record must be created, or a finding waived: `diff.source-requires-record`, and any rule whose violation is intentional.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. A prose `suggested_action` string | Trivial to add to every rule | Not machine-actionable; an agent must parse English; contradicts `ADR-64` |
| B. A structured object with a closed `kind` vocabulary, a `target` and an optional `value`, declared per rule in the rule table (chosen) | Agents dispatch on `kind`; the vocabulary is finite and testable; `fix` and `check` can share it; coverage is enforceable | The vocabulary is a public contract; adding a kind later is additive, renaming one is breaking |
| C. An open `kind` string with free-form `params` | No up-front vocabulary design | Consumers cannot dispatch; each rule invents its own; no coverage test is meaningful |
| D. Only populate the mechanically obvious rules (`MILE-44`'s What) and omit the rest | Smallest change | A consumer cannot tell "no action" from "not yet written"; the coverage test cannot demand completeness |

## Decision

In the context of findings that say what is wrong but never what to do, facing an output contract that agents parse and that `ADR-64` requires to be structured, we decided that **every finding carries a `suggested_action` object with a `kind` drawn from a closed vocabulary, a `target`, and a `value` only when the engine computed one**, to achieve findings an agent can act on without reading prose, accepting that the vocabulary is a public contract.

Proposed shape, serialized as `suggested_action` (snake_case like the rest of the report; `SPEC-2` has no camelCase fields, so `MILE-44`'s `suggestedAction` spelling is corrected here):

```json
{ "kind": "edit-field", "target": { "file": "docs/bugs/BUG-9-x.md", "field": "Regression-test" }, "value": null, "note": "..." }
```

- `kind` is one of: `fix-apply` (a computed repair that `urzua fix --apply` performs; `target` names the record and field, `value` is the computed value), `edit-field` (set, change or remove a header field; `value` present only when computed, absent when a human must choose), `add-record` (create a record; `target.type` names the declared type), `edit-section` (add or fill a named section), `edit-config` (change a named `.urzua/config.yaml` key), `waive` (the finding may be waived by a waiver record; `target` carries rule and file). The vocabulary is closed: a kind outside it is a test failure.
- `target` is a structured object whose keys depend on `kind` and are declared once with the kind.
- `value` is present only when computable; its absence means the engine does not know, never that the action is optional.
- `note` is an optional short human-readable sentence and is not read by the engine; the structure alone must be sufficient.
- Each rule declares its action in `RULE_TABLE` (`MILE-171`): the kind, and a function from the finding's context to target and value. A rule with no action is an explicit declared exemption stating why, not an omission. A coverage test fails when a rule has neither.
- A finding with no applicable action omits the field; it is never `null` or an empty object (consistent with `waived`).

## Reversibility

Adding a kind or an optional key is additive and cheap. Renaming a kind, changing `target`'s keys, or moving from object to another type is a breaking change for every parser of `check` output, and the output contract has no version field to negotiate it. The vocabulary should therefore be reviewed before acceptance, and the first release should use the smallest set that covers the rule groups above.

## Consequences

- `Finding` gains `suggested_action`; `SPEC-2` documents it with a revision row and its output contract block, and the acceptance expectations (`MILE-170`) change for every finding that now carries one.
- Every rule constructor and `RuleDef` gains an action declaration; a rule shipped without one fails the coverage test.
- `urzua fix` detect output can reuse `fix-apply` rather than a parallel shape, which `RFC-8` and `ADR-15` require.
- `MILE-72` (a top-level `next_action`) can reuse the same kinds.
- `MILE-44`'s Done means ("a rule without one omits it") conflicts with the coverage test the maintainer asked for; this ADR resolves it as: omission is allowed only through a declared exemption. `MILE-44` needs a revision row when this is accepted.
- Cost: designing 53 actions is real work, and some (a human must write a justification) carry no `value`, so the field is a pointer to the right edit, not a patch.

## References

- `ADR-64` -- structured in, structured out.
- `ADR-15` and `RFC-8` -- name `suggested_action` on `fix` detect entries.
- `RFC-3` / `ADR-7` -- agent-native JSON output.
- `MILE-44`, `MILE-171`, `MILE-72`, `BUG-21`, `SPEC-2`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed. **Why:** `MILE-44` needs a structured shape that no accepted record defines and that is hard to reverse once agents parse it. | **substantive** |
