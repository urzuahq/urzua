---
Stable-Id: 01M6MC453YYMEB171A1F1G5MC1
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-48
---
# 75 — An acyclic relation pair is a sequencing relation, checked by two sequence rules

## Context

`RFC-48` (Accepted) proposes a sequencing relation: `Depends-on` with the inverse `Blocks`, declared
through `RFC-47`'s `relation_pairs`, with one new key, `acyclic`, and rules that report a cycle and a
value that names no record. `MILE-27` builds that part, and `MILE-182` (`urzua roadmap`) and `MILE-144`
(the derived `blocked` value) wait on it. Building part of an RFC needs its own decision record, as
`ADR-65` did for `RFC-47`: the RFC fixes the key and the two rule ids, and leaves the readings below
open. `MILE-27` builds the narrowest reading of each and `SPEC-3` records them; this ADR makes them
decisions rather than implementation.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| What `acyclic` means for the inverse field | Edges from the forward field only vs. both fields, each in its declared direction | A record that writes only `Blocks` is stating a dependency; ignoring it hides a loop. Both fields, direction by field name |
| Which records a cycle is reported on | One finding per cycle vs. every record on it | A diff-scoped run examines only the records it changed; one finding anchored on one member can be scoped away. Every member, each with a shortest cycle through it |
| What a claim to nothing does to the graph | Abort vs. no edge | An unresolved claim is `sequence.unresolved-blocker`'s; it must not hide a real cycle among resolved claims. No edge |
| Whether prose in the field is a finding | Ignore non-references vs. report them | A dropped entry orders nothing and says nothing (ADR-7). Reported by `sequence.unresolved-blocker`; the `condition` record type is how a fact that is not work becomes a reference |
| Rule scope | Every pair vs. acyclic pairs only | Only a pair declared a sequencing relation is the rules' business; a non-acyclic pair examines nothing |
| An acyclic rule with no acyclic pair | Run on nothing vs. refuse at load | `ADR-65` refuses a pair rule with no pair; the same reasoning applies |
| Relation to `reciprocity` | `acyclic` implies a value vs. independent | `RFC-48` uses `reciprocity: none` for `Depends-on` and says an adopter may set `required`; independent |

## Decision

In the context of `RFC-48` section 1, facing a sequencing relation to build before `urzua roadmap` can
exist, we decided that `acyclic` is an independent boolean on a `relation_pairs` entry (default
`false`); that `sequence.cycle` draws one edge per resolved claim in either field in the direction the
field declares and reports every record on a cycle with a shortest cycle through it; that
`sequence.unresolved-blocker` reports a reference that resolves to nothing and an entry that is not a
record reference; that both rules read acyclic pairs only; and that enabling either with no acyclic
pair declared is a load error. We accept that a value reports under more than one rule when the field
is also a `pointer_fields` entry, and that a prose value in a sequencing field is an error rather than
a tolerated note.

## Reversibility

Moderate. `acyclic`, `sequence.cycle` and `sequence.unresolved-blocker` are named by `RFC-48` and become
public once released; the readings above can be loosened or tightened by an amendment without renaming
anything, but a config that adopted the key cannot lose it.

## Consequences

- Any pair can be declared acyclic, not only a dependency pair; the rule ids keep the `sequence.` prefix
  `RFC-48` gave them.
- `MILE-182` reads the same edges the cycle rule checks, so a roadmap is computed only from a graph
  `check` has already accepted.
- `RFC-48`'s other rules (`sequence.phase-order`, `sequence.failed-dependency`,
  `sequence.terminal-parent-open-child`) are not decided here.
- A record on a cycle is reported once per pair even when it wrote none of the claims that close it.

## References

`RFC-48`, `RFC-47`, `ADR-65`, `ADR-64`, `MILE-27`, `MILE-182`, `MILE-144`, `SPEC-2`, `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed. **Why:** `MILE-27` builds part of `RFC-48`, and building part of an RFC needs its own decision record; the maintainer approves before it is Accepted. | **substantive** |
