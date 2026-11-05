---
Stable-Id: 01M6N33XTAZBF07CBH3RPDB75W
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-48, ADR-76
---
# 80 — urzua roadmap computes waves, the ready set and the critical path over acyclic relation pairs

## Context

`RFC-48` (Accepted) proposes `urzua roadmap`: waves, a ready set, a critical path, what each record
blocks, and a grouping. `MILE-182` builds it. `ADR-76` decided the sequencing relation and its two
rules, and says so: it does not decide `RFC-48`'s other parts, and it leaves "a loop across two acyclic
pairs" to the roadmap. `MILE-144` (PR for `sequence.status-agreement`) built the derived `blocked`
value from `ADR-64` clause 4 and `ADR-76`'s edges. Nothing accepted decides what the roadmap output
means, and building part of an RFC needs its own decision record (`docs/rfc` rule, as `ADR-65` and
`ADR-76` did).

The RFC states the outputs in a sentence each and leaves readings that change what a consumer sees. A
dashboard or agent will be written against the shape, and the shape is a public contract like
`ADR-73`'s. The readings below were found by working the `MILE-182` fixtures by hand (a failure-terminal
blocker, a cycle, a relation that matches nothing, no acyclic pair).

Evidence on this repository's own corpus at 2026-11-05: exactly one record carries `Depends-on`
(`MILE-182`, naming `MILE-27` and `MILE-144`, both Done once `MILE-144` lands). Of the 151 milestone
`Blocked-on` values, 145 are the `—` placeholder, one is blank, three are prose about a fact or a
decision, and two name a record (`MILE-178` on `MILE-12`, `MILE-182` on `MILE-144`). A roadmap computed today would hold one record, ready, with a critical path of
length one. The command is correct and not yet informative; the data migration is separate work.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| Which relations | One roadmap per acyclic pair vs. one over the union of all acyclic pairs | A union needs a direction rule for pairs that are not dependencies (containment). `ADR-76` already refuses a cross-pair graph. Per pair, named by the pair, so a repository with `sequencing` and a second acyclic pair gets two roadmaps and no cross-pair claim |
| What is open | Any record not in a terminal status vs. only records with a declared status vocabulary | A record whose type declares no `success` cannot be judged finished. Open = its status is in neither `success` nor `failure` of its own type. A record of a type that declares neither list is open and is disclosed in a `notes` entry |
| Wave | Longest-path layer vs. earliest-start layer | Wave of an open record = 0 when it has no open dependency, else 1 + the largest wave among its open dependencies. A dependency in `success` is ignored. This is the RFC's "topological layers over open records" with the layering fixed |
| A dependency in a failure terminal | Treat as done vs. treat as open vs. exclude the dependent | RFC-48: it does not unblock. The dependent is not ready, is placed in no wave, and is listed under `needs_replan` with the failed dependency (the data `sequence.failed-dependency` reports). A record that depends on a `needs_replan` record is excluded too, transitively |
| Ready | Wave 0 vs. wave 0 and not needing re-plan | Open, every dependency in `success`. Equal to wave 0 minus nothing, since `needs_replan` records are in no wave. Written `Status` is ignored: readiness is derived |
| Critical path | Longest chain by count vs. by weight; one path vs. all ties | There are no estimates (RFC non-goal), so count. The longest chain of open records following dependencies; ties are broken by the smallest record ID at each step (natural order of prefix, then number) so the answer is deterministic. One path, in dependency order, blocker first. Computed over the whole roadmap, not per group |
| Transitive blocks | Per record vs. omitted | Per open record, the sorted open records that transitively depend on it. Terminal records are not listed |
| Cycles | Abort vs. report and exclude | `sequence.cycle` is the check; the roadmap must not loop. Records on a cycle, and records that depend on one, are excluded from waves and listed under `cycles` (members) and `needs_replan` reason `cycle`. A cycle is reported, never an error exit by itself |
| A relation that matches nothing | Empty roadmap vs. report | An acyclic pair no record carries a value for, an unresolved claim, and a claim on an unreadable record are listed under `notes` with the pair and field. An empty result never stands in for "nothing declared matched" |
| No acyclic pair | Empty roadmap vs. explicit statement | `{"roadmap": null, "reason": "no relation_pairs entry declares acyclic: true"}`, exit 0. Not an empty roadmap |
| Grouping | Hard-coded `Phase`/`Track` vs. declared | Top-level config `roadmap.group_by`, an ordered list of field names (default: none, so the output holds no `groups`). The engine names no field. A group lists record IDs in wave order. A configured field no record carries is a `notes` entry |
| Output shape | Free vs. fixed here | Fixed here, reproduced verbatim in `SPEC-2` (command) and `SPEC-3` (config) and embedded under `roadmap` in `export --format json` (`MILE-115`) |

## Decision

*In the context of `RFC-48`'s accepted but unspecified `urzua roadmap`, facing a public output shape and
several readings that change what a consumer sees, we decided that the roadmap is computed per acyclic
relation pair, over open records (a status in neither the type's `success` nor `failure`), using the
same edges `sequence.cycle` checks (`ADR-76`); that a record's wave is 0 with no open dependency and
otherwise 1 plus its deepest open dependency; that the ready set is the open records whose every
dependency is in `success`; that a dependency in a failure terminal, or on a cycle, keeps the dependent
(and anything depending on it) out of the waves and the ready set and lists it under `needs_replan`; that
the critical path is the longest chain of open records by count, ties broken by smallest record ID; that
grouping fields come from `roadmap.group_by` in configuration; that every declared input that matched
nothing is a `notes` entry, and a repository with no acyclic pair gets an explicit `roadmap: null` with
a reason; and that output is JSON only, with its shape fixed in `SPEC-2`, to achieve a deterministic,
tracker-free order of work, accepting that the critical path ignores effort and that the command reports
little until records carry `Depends-on`.*

Proposed shape (one object per pair, under `roadmap.pairs`):

```json
{
  "roadmap": {
    "pairs": [
      {
        "pair": "sequencing",
        "waves": [["MILE-5"], ["MILE-7", "MILE-8"]],
        "ready": ["MILE-5"],
        "critical_path": ["MILE-5", "MILE-7"],
        "blocks": {"MILE-5": ["MILE-7", "MILE-8"]},
        "groups": [{"field": "Phase", "values": {"0": ["MILE-5", "MILE-7"]}}],
        "needs_replan": [{"record": "MILE-9", "reason": "failed-dependency", "via": "MILE-3"}],
        "cycles": [["MILE-10", "MILE-11"]],
        "notes": []
      }
    ]
  }
}
```

Open for the maintainer in review: whether `groups` nests (`Phase` then `Track`) as the RFC says or stays
a flat list per field as drawn; whether a record of a type with no status vocabulary should be open or
out of scope; and the `roadmap.group_by` key name.

## Reversibility

Moderate to hard once consumers parse it. Key names and the wave definition are public contract. The
tie-break and the exclusion of failure-dependents are the readings most likely to be argued, and
loosening them (including more records) is additive; changing a wave number or the critical path is
breaking for a consumer. Decided before the command ships for that reason.

## Consequences

- `MILE-182` is blocked on this ADR's acceptance; nothing of the roadmap is built until then.
- `urzua roadmap` reuses `sequence_graphs` (introduced with `sequence.status-agreement`), so it reads
  the graph `check` accepts, with `ADR-76`'s two blind spots.
- `SPEC-2` gains the command and its output shape, `SPEC-3` the `roadmap` key.
- This repository's roadmap is nearly empty until its `Blocked-on` values that name a record, and
  the prose waits that need the `condition` type, are migrated to `Depends-on`. That migration needs
  human judgement about which waits are real dependencies, and is not done here.
- `sequence.failed-dependency`, `sequence.phase-order` and `sequence.terminal-parent-open-child` remain
  undecided; `needs_replan` carries the data the first would report but the rule is not decided here.

## References

`RFC-48`, `ADR-76`, `ADR-64`, `ADR-73`, `MILE-182`, `MILE-144`, `MILE-115`, `SPEC-2`, `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** `MILE-182` builds `RFC-48`'s `urzua roadmap`, no accepted record decides its waves, ready set, critical path or output shape, and an agent may not accept it; the maintainer decides. | **substantive** |
