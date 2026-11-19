---
Stable-Id: 01M7T32TYW7X5Y5J60EAWKW8FH
Status: Proposed
Date: 2026-11-19
Author: beauwilliams
Deciders: beauwilliams
Embodiment:
Supersedes / Superseded-by:
---
# 104 — A rule entry lists the record types it judges

## Context

A rule judges every record type its own logic reaches, and nothing in a rule entry under `rules:` narrows that. `status.children-agreement` (`MILE-348`) applies to every type that declares a `part_of: true` pointer field. In this repository only specs do, and specs are living documents whose `Parent` is containment of current truth, not planned work moving toward done, so the rule's not-started and finished-with-open-child verdicts do not fit them. The rule cannot be enabled for the types where it fits without also judging the ones where it does not: there is no way to say "this rule, only these types". The same gap exists for every record-level rule, because the only per-rule narrowing the engine had was `scope: changed` (`ADR-90`), which narrows by the diff, not by type.

The direction comes from the maintainer, on 2026-11-19, choosing "types: on the rule entry" among offered options: "You should be able to set it. The rules per record type, right? Like, that should be a thing for sure."

## Options considered

| Option | Pros | Cons |
|---|---|---|
| `types:` on the rule entry | One key, read the same way for every rule; the rule entry already holds the rule's options (`ADR-64`); absent means today's behaviour | A rule that judges no record of a type has nothing to narrow, so the key must be refused there |
| A per-type rule list under `record_types` | Reads naturally from the type's side | Splits a rule's configuration across two places, and a rule's level and options would still live under `rules:` |
| A per-rule option on the rules that need it | Smallest change for the motivating rule | Reimplemented per rule; the next rule that needs it repeats the work, against the config-driven thesis (`SPEC-1`) |

## Decision

In the context of a rule entry that cannot say which record types its rule judges, facing a rule (`status.children-agreement`) that fits some types and not others, we decided that any rule entry in the map form may carry `types:`, a list of declared record-type names, to achieve a rule enabled exactly where it fits, accepting that a rule which judges no record of a type refuses the key at load.

```yaml
rules:
  status.children-agreement:
    level: error
    types: [milestone]
  header.required-fields: error   # all types, as today
```

Absent, the rule judges every type, exactly as before. A record of a type the list does not name is not judged by that rule: it is counted in the rule's population as out of scope, and a finding on it is not reported. The rule still reads every record as context, so a rule that judges a record against others (a parent against its children, a reference against its target) reads the same records it read before.

## Reversibility

Easy to walk back while no configuration uses the key: it is opt-in, absent by default, and a configuration without it produces byte-identical output. Once adopters use it, removing it would make their configurations fail to load, so it would be deprecated rather than removed.

## Consequences

- **Load errors.** A name in `types:` that is not a declared record type is a load error naming the rule and the type. An empty list is a load error, and so is the key present with no value, as for every rule option. A rule that judges no record of a type refuses the key at load with `rule '<id>' does not take the option 'types'` and a reason.
- **The notice.** A listed type that the rule can never judge however its records are edited is not an error, but `check` names every such entry in one `rule-types-never-judged` warning notice, so the setting is never silently a no-op. It is read from the type's declaration: a type declaring `header_shape: none` for a rule that reads a header, a type no `part_of: true` field names as a target for `status.children-agreement`, and a type that does not declare the `sections` key a section rule reads. A type with no records yet is not reported. In this repository's own configuration, `status.children-agreement` with `types: [milestone]` would be named in that notice, because no `part_of` field targets milestones.
- **Population.** A candidate whose record's type is not listed is counted in `out_of_scope`, and also in an additive `out_of_types` counter that is serialized only when non-zero, so a run that examined nothing because of `types:` is told apart from a run that found nothing, and a configuration without the key reports exactly what it did before. `config.scope-matches-nothing` does not read those candidates as a configuration that reaches nothing, the way it already excludes headerless types.
- **One mechanism.** The runner applies the list for every rule: the census that builds a rule's population counts a record of an unlisted type out of types, and the runner drops the rule's findings on such records. No rule carries its own filter. The same predicate (`Config::rule_judges_type`) narrows what `fix` repairs for `embodiment.consistency`, `status.delivery-agreement` and the reciprocity rules, so `fix` never repairs what `check` does not report. It holds the same way on a plain run, `check --base`, the editor hook's `--stdin-path`, and composes with `scope: changed` and every other option; `urzua explain --rule` shows the declared list.
- **Per-category decisions** (by the rule table's `scope` and `anchor`, decided by the builder, pending maintainer review): a rule over loaded records whose findings sit on a record (`scope: records`, anchored on a record or a record-or-declared-file) takes `types:`, including cross-record and changed-set rules; a rule that judges the configuration (`scope: config`), files that are not records (`scope: files`), a diff's changed paths (`scope: diff`) or the run itself (`scope: run`) refuses it, and so does `filename.identity-pattern`, whose findings sit on a type's directory. Narrowing a directory-anchored rule by type is a possible later extension, not built.

## References

- `SPEC-1` (config-driven, no silent no-op), `SPEC-2` (population counters), `SPEC-3` (configuration).
- `ADR-53` and `ADR-87` (populations and levels), `ADR-64` (rule options), `ADR-90` (`scope: changed`).
- `MILE-348` (`status.children-agreement`), `MILE-351` (delivers this decision).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-19 | Filed as Proposed. **Why:** the maintainer's direction of 2026-11-19 quoted in Context. | **substantive** |
