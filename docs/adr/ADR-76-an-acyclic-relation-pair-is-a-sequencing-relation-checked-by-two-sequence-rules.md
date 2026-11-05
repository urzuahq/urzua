---
Stable-Id: 01M6MC453YYMEB171A1F1G5MC1
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-47, RFC-48
---
# 76 — An acyclic relation pair is a sequencing relation, checked by two sequence rules

## Context

`RFC-48` (Accepted) proposes a sequencing relation: `Depends-on` with the inverse `Blocks`, declared
through `RFC-47`'s `relation_pairs`, with one new key, `acyclic`, and rules that report a cycle and a
value that names no record. `MILE-27` builds that part, and `MILE-182` (`urzua roadmap`) and `MILE-144`
(the derived `blocked` value) wait on it. Building part of an RFC needs its own decision record, as
`ADR-65` did for `RFC-47`: the RFC fixes the key and the two rule ids, and leaves the readings below
open. This ADR makes the readings decisions rather than implementation. Where one departs from what
`RFC-48` says, the departure is named.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| What `acyclic` means for the inverse field | Edges from the forward field only vs. both fields, each in its declared direction | A written `Blocks` states a dependency, and forward-only reading would hide a loop through it. Both fields, direction by field name. **This departs from `RFC-48`**, which calls `Blocks` derived and never written: `Blocks` is declared only because `RFC-47` refuses a pair whose field no type declares. A repository following `RFC-48` writes `Depends-on` only; a written `Blocks` is allowed, not expected, and `RFC-48`'s derived `Blocks` (`MILE-182`, `MILE-144`) is the union of the computed inverses and any written `Blocks` |
| Which records a cycle is reported on | One finding per cycle vs. every record on it | A diff-scoped run examines only the records it changed; one finding anchored on one member can be scoped away. Every member, each with a shortest cycle through it. Findings for a large cycle are redundant by design, and the cost is linear in the component. This is the loudest reading, not the narrowest |
| What a claim to nothing does to the graph | Abort vs. no edge | An unresolved claim is `sequence.unresolved-blocker`'s and must not hide a real cycle among resolved claims. No edge. The same holds for an edge into a record whose header cannot be read: it is dropped, so a cycle through that record is not seen; the record is disclosed in the rule's population |
| What counts as a prose entry | Any non-reference text vs. only text with no leading reference | The value is split on commas; an entry is a reference only when the whole trimmed entry is one record reference. `MILE-4 MILE-5` and `MILE-5 (see note)` are prose, because accepting a leading reference would drop the rest from the graph unseen |
| Whether prose is a finding | Ignore non-references vs. report them | A dropped entry orders nothing and says nothing (`ADR-7`). Reported by `sequence.unresolved-blocker`; the `condition` record type is how a fact that is not work becomes a reference. This is wider than `RFC-48`'s "a value that does not resolve" |
| Overlap with the pointer rules | Report twice (as `ADR-65` accepted for `relation.pair-reciprocity`) vs. yield to them | Two blocking errors per defect, two waivers to silence one, and two findings with different suggested actions for one edit. The pointer rules are the general, lower layer, so the sequence rule yields: it skips an unresolved target when the field is one of the type's `pointer_fields` and `pointer.resolution` is enabled, and a prose entry when `header.pointer-field-clean` is enabled. In a repository following `RFC-48` (`Depends-on` a pointer field, both rules on) it then never fires; it is a guard that the graph the roadmap is computed from lost no claim, and it reports in a repository that does not enable the pointer rules or does not declare the field a pointer field |
| Cycles across pairs | One graph per pair vs. one graph over all acyclic pairs | A loop crossing two acyclic pairs (for example `Depends-on` and a containment pair) is not detected by `sequence.cycle`; a graph over several pairs would need a decision about their direction that nothing yet needs. Left to the roadmap |
| Rule scope | Every pair vs. acyclic pairs only | Only a pair declared a sequencing relation is the rules' business; a non-acyclic pair examines nothing |
| An acyclic rule with no acyclic pair | Run on nothing vs. refuse at load | `ADR-65` refuses a pair rule with no pair; the same reasoning applies |
| Relation to `reciprocity` | `acyclic` implies a value vs. independent | `RFC-48` uses `reciprocity: none` for `Depends-on` and says an adopter may set `required`; independent |

## Decision

In the context of `RFC-47` and `RFC-48` section 1, facing a sequencing relation to build before
`urzua roadmap` can exist, we decided that `acyclic` is an independent boolean on a `relation_pairs`
entry (default `false`); that `sequence.cycle` draws one edge per resolved claim in either field in the
direction the field declares, so a written `Blocks` is an edge, and reports every record on a cycle with
a shortest cycle through it; that `sequence.unresolved-blocker` reports a reference that resolves to
nothing and an entry that is not, as a whole, one record reference, except what an enabled pointer rule
already reports for a pointer field; that both rules read acyclic pairs only and one graph per pair; and
that enabling either with no acyclic pair declared is a load error. We accept that a loop across two
pairs and a loop through an unreadable record are not seen, that a prose value in a sequencing field is
an error rather than a tolerated note, and that the unresolved-blocker rule is silent in a repository
that follows `RFC-48`.

## Reversibility

Moderate. `acyclic`, `sequence.cycle` and `sequence.unresolved-blocker` are named by `RFC-48` and become
public once released. Most readings can be loosened later without breaking an adopter: fewer cycle
findings, prose tolerated, a rule that no longer refuses to load. The one awkward to undo is that a
written `Blocks` is an edge: removing it makes cycles through a written `Blocks` pass silently, and adding
it later raises new errors in configs that were green. It is decided now for that reason. Narrowing the
overlap later would turn a waiver naming `sequence.unresolved-blocker` into `waiver.matches-nothing`, so
it is settled before waivers exist.

## Consequences

- Any pair can be declared acyclic, not only a dependency pair; the rule ids keep the `sequence.` prefix
  `RFC-48` gave them.
- `MILE-182` reads the same edges the cycle rule checks, so a roadmap is computed only from a graph
  `check` has already accepted, apart from the two blind spots named above.
- `RFC-48`'s other rules (`sequence.phase-order`, `sequence.failed-dependency`,
  `sequence.terminal-parent-open-child`) are not decided here.
- A record on a cycle is reported once per pair even when it wrote none of the claims that close it.

## References

`RFC-47`, `RFC-48`, `ADR-65`, `ADR-64`, `MILE-27`, `MILE-182`, `MILE-144`, `SPEC-2`, `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Drafted as Proposed. **Why:** `MILE-27` builds part of `RFC-48`, and building part of an RFC needs its own decision record; the maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-04 | Amended while Proposed: the departure from `RFC-48` on a written `Blocks` is stated; the unresolved-blocker rule yields to the pointer rules; a prose entry is defined as an entry that is not, as a whole, one record reference; the cross-pair and unreadable-record blind spots are stated; `RFC-47` added to `Derives-from`. **Why:** design review of this ADR and of the first implementation, which read only the first token of an entry. | **substantive** |
