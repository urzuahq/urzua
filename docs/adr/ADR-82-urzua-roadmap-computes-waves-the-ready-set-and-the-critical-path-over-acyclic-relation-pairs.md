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
# 82 — urzua roadmap computes waves, the ready set and the critical path over acyclic relation pairs

## Context

`RFC-48` (Accepted) proposes `urzua roadmap`: waves, a ready set, a critical path, what each record
blocks, and a grouping. `MILE-182` builds it. `ADR-76` decided the sequencing relation and its two
rules, and says it does not decide `RFC-48`'s other parts; it leaves "a loop across two acyclic pairs"
to the roadmap. A separate decision record (in draft, by subject: the derived `blocked` value and
its agreement with a written `Status`) decides that `blocked` is derived only, over the same edges.
Nothing accepted decides what the roadmap output means, and building part of an RFC needs its own
decision record (as `ADR-65` and `ADR-76` did).

The output is a public contract that dashboards and agents parse, like `ADR-73`'s. The readings below
were found by working the `MILE-182` fixtures by hand (a failure-terminal blocker, a cycle, a relation
that matches nothing, no declared sequencing pair), and were amended after design review.

Evidence on this repository's own corpus at 2026-11-05: exactly one record carries `Depends-on`
(`MILE-182`). Of the 151 milestone `Blocked-on` values, 145 are the `—` placeholder, one is blank,
three are prose about a fact or a decision, and two name a record. A roadmap computed today holds
almost nothing, and every open record with no `Depends-on` is wave 0 even when it waits on a prose
fact. The command is correct and not yet informative; the migration is separate work, and
`unstructured-wait` notes (below) make the gap visible until the `condition` record type exists.

## Options considered

| Question | Options | Decision driver |
|---|---|---|
| Which pairs | Every `acyclic` pair vs. a declared set | A containment pair (`Parent`/`Children`) may be `acyclic` (`ADR-76`) but is not a dependency: a child does not wait on its parent finishing. The set is declared, `roadmap.pairs: [name, ...]`, each naming a `relation_pairs` entry that is `acyclic`. Validated at config load. No `roadmap.pairs` means no roadmap, stated as a note |
| One graph or one per pair | Per pair vs. one graph over the declared pairs | `ADR-76` left a loop that crosses pairs to the roadmap, and per-pair output would pass it through undetected. **One graph over all declared pairs**, edges drawn as `sequence.cycle` draws them. A loop that crosses declared pairs is reported as a cycle. `blocks`, waves and the critical path are over that one graph |
| What is open | Terminal status excluded vs. only records with a vocabulary | Open means the record's status is in neither `success` nor `failure` of its own type. A record of a type that declares no status vocabulary, or with no `Status` in a type that has one, **is open** and gets a `no-status-vocabulary` note. Open is the larger set, so it is the safe one to start with (shrinking it later removes records from output); #313's derived value already treats such a dependency as blocking |
| Ready | Wave 0 vs. open and not derived-blocked | Ready means open **and not derived-blocked**, the derived `blocked` value being computed over the same declared pairs. It is wave 0 by construction, since a record with no open dependency has none that block it; both are emitted and neither is to be "fixed" to the other. Written `Status` is ignored: `Blocked` is derived only (`RFC-48`) |
| Wave | Longest-path layer vs. earliest-start | Wave of an included open record = 0 with no open dependency, else 1 + the largest wave among its open dependencies; a dependency in `success` is ignored. Unique, so no tie |
| A dependency in a failure terminal | Treat as done vs. open vs. exclude | It does not unblock (`RFC-48`). The dependent and everything that transitively depends on it is excluded from waves, `ready` and the critical path, and listed with a `needs_replan` reason. The way out is to re-point the `Depends-on` (a `Superseded` target is re-pointed at its successor, the common case); the record then re-enters a wave |
| Critical path | Longest by count vs. weight; greedy vs. global; whole vs. included | No estimates (`RFC` non-goal), so count. Over included records only: the longest chain; among all longest chains, **the lexicographically smallest sequence of ids, blocker first** (not a greedy walk, which can take a shorter branch). Emitted once, for the one graph. A chain hidden behind an excluded record is visible in `needs_replan`, not in the path |
| Ordering | Unspecified vs. one rule | **Natural id order** everywhere: prefix compared as text, then the number compared as a number, then the whole id as text for an id with no number. Every list of ids, every map key set, `waves` inner arrays, `ready`, `blocks`, `blocked_by`, group members, `needs_replan` and `cycles` entries are ascending in it. `notes` are ordered by `kind` (the closed set's order below), then `record`, then `pair` |
| Cycles | Abort vs. report and exclude | Each strongly connected component of size above one, or a record with an edge to itself, is listed once as an array of its members in ascending order, which starts at the smallest member. Members, and records that depend on them, are excluded and appear in `needs_replan` with reason `cycle` / `depends-on-cycle` |
| Relation matching nothing | Empty result vs. report | A declared pair no record carries a value for, an unresolved claim, and a claim on an unreadable record are `notes`. An empty result never stands for "nothing declared matched" |
| No declared pair | `null` vs. object | Always an object. No `roadmap.pairs` gives `records: {}`, empty lists and a `no-sequencing-pair` note |
| Grouping | Flat map vs. nested array | Declared `roadmap.group_by`, an ordered list of field names; the engine names none. Nested in `group_by` order as arrays; a record with no value for a field goes in a bucket with `value: null`, emitted last. A name no type declares is a config load error (`ADR-64` clause 5); a field no open record carries is a note |
| Shape | Ids only vs. a per-record map | A `records` map keyed by id carries the per-record data, so a page needs no join for the common fields |

## Decision

*In the context of `RFC-48`'s accepted but unspecified `urzua roadmap`, facing a public output and
readings that change what a consumer sees, we decided that the roadmap is one graph over the
sequencing pairs named by `roadmap.pairs`, using the edges `sequence.cycle` checks (so a loop that
crosses declared pairs is found); that wave, ready, critical path, ordering and cycle listing are as
the table states; that a type with no status vocabulary is open and noted; that a failed or cyclic
dependency keeps a record and its dependents out of the waves under a closed `needs_replan` reason; that
ready means open and not derived-blocked; that output is JSON only, always an object, versioned by
`schema`, with structured notes from a closed set; and that `export --format json` carries the roadmap
only when `roadmap.pairs` is declared, to achieve a deterministic, tracker-free order of work, accepting
that the critical path ignores effort and the command reports little until records carry `Depends-on`.*

Output (all keys snake_case; `schema` is an integer, 1 in the first release):

```json
{
  "roadmap": {
    "schema": 1,
    "pairs": ["sequencing"],
    "waves": [["MILE-5"], ["MILE-7", "MILE-8"]],
    "ready": ["MILE-5"],
    "critical_path": ["MILE-5", "MILE-7"],
    "records": {
      "MILE-5": {"type": "milestone", "status": "Planned", "wave": 0, "ready": true,
                 "blocked": false, "blocked_by": [], "blocks": ["MILE-7", "MILE-8"]},
      "MILE-9": {"type": "milestone", "status": "Planned", "wave": null, "ready": false,
                 "blocked": true, "blocked_by": ["MILE-3"], "blocks": []}
    },
    "groups": [
      {"field": "Phase", "value": "0", "groups": [
        {"field": "Track", "value": "x", "records": ["MILE-5", "MILE-7"]},
        {"field": "Track", "value": null, "records": ["MILE-8"]}]},
      {"field": "Phase", "value": null, "groups": []}
    ],
    "needs_replan": [{"record": "MILE-9", "reason": "failed-dependency", "via": "MILE-3"}],
    "cycles": [["MILE-10", "MILE-11"]],
    "notes": [{"kind": "unstructured-wait", "record": "MILE-30", "pair": null, "field": "Blocked-on",
               "message": "..."}]
  }
}
```

- `blocks` and `blocked_by` are transitive and direct respectively: `blocked_by` the direct open or
  failed dependencies, `blocks` every open record that transitively depends on this one. `blocked` is the
  derived value. `wave` is `null` for a record excluded by `needs_replan`. `status` is the written value.
- `needs_replan.reason` is a closed set: `failed-dependency` (a direct dependency is in a failure
  status; `via` is that dependency, the smallest id if several), `depends-on-excluded` (an indirect one:
  `via` is the nearest excluded dependency, smallest id if several, and the root cause is reachable by
  following `via`), `cycle` (a member of a cycle; `via` is `null`), `depends-on-cycle` (`via` is the
  nearest cycle member or excluded record, smallest id).
- `notes[].kind` is a closed set, in this order: `no-sequencing-pair`, `pair-matches-nothing`,
  `unresolved-claim`, `unreadable-record`, `no-status-vocabulary`, `unstructured-wait`, `group-field-unset`,
  `nothing-open`. `unstructured-wait`: an open record with a non-placeholder narrative `Blocked-on` and no
  `Depends-on`; it stays until the `condition` record type lands. `nothing-open` distinguishes "all
  finished" (`waves: []`) from "nothing declared". Adding a kind, key or `reason` is non-breaking;
  consumers treat an unknown one as informational, as `ADR-73` rules for `kind`.
- **Departures from `RFC-48`'s sketch, named:** the sketch has `groups` keyed by field and a per-record
  `derived` object (`blocked`, `blocks`, `children`) on exported records. Here `groups` is a nested array,
  and the derived values live in the `records` map beside `type` and `status`, with no `children`
  (containment is not a sequencing pair here; a containment view is a later decision). The roadmap is
  computed over a declared set of pairs, not every acyclic pair.
- `export --format json` carries the `roadmap` key only when `roadmap.pairs` is declared; a repository
  that has not opted in sees no new key.

## Reversibility

Hard once consumers parse it, and the shape is the part to settle now. In order: the wave definition and
the position of ids in `waves` (changing a number is breaking); the output shape (nested arrays,
always-an-object, `records`); the open set (shrinking it removes records from output, growing it is
additive, hence the larger set); the critical-path tie-break (silent when it changes); and the
`roadmap.pairs` and `roadmap.group_by` keys (a rename is a config migration under `ADR-77`). Adding a
note kind, a `reason` or a key is additive.

## Consequences

- `MILE-182` is blocked on this ADR's acceptance; nothing of the roadmap is built until then. When this
  ADR is accepted, the acceptance change must also move `MILE-182`'s `Status`, or the derived-blocked
  agreement rule reports its written `Blocked`. That is the cost of keeping a hand-written `Blocked`.
- A `Rejected`, `Superseded` or `Retired` dependency (this ADR included) sends its dependent to
  `needs_replan`, which is the intended reading of "the design we waited on was refused".
- `urzua roadmap` reuses `sequence_graphs`, so it reads the graph `check` accepts, with `ADR-76`'s
  blind spot for a record whose header cannot be read, which is a note here. The cross-pair blind spot
  `ADR-76` named is closed for the declared pairs.
- `SPEC-2` gains the command and its output, `SPEC-3` the `roadmap` key with load-time validation of
  `pairs` and `group_by`.
- `sequence.failed-dependency`, `sequence.phase-order` and `sequence.terminal-parent-open-child` remain
  undecided; `needs_replan` carries the data the first would report.
- This repository's roadmap stays near-empty until its record-naming waits move to `Depends-on` and the
  prose waits to condition records.

## References

`RFC-48`, `ADR-76`, `ADR-64`, `ADR-73`, `ADR-77`, `MILE-182`, `MILE-144`, `MILE-115`, `SPEC-2`, `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** `MILE-182` builds `RFC-48`'s `urzua roadmap`, no accepted record decides its waves, ready set, critical path or output shape, and an agent may not accept it; the maintainer decides. | **substantive** |
> | 2026-11-05 | Amended while Proposed after design review: one graph over the pairs `roadmap.pairs` declares (a cross-pair loop is detected); ready means open and not derived-blocked; natural id ordering, the critical-path tie-break and the cycle listing fixed; `needs_replan` reasons and `via` defined; output always an object with `schema`, a per-record map and structured notes from a closed set including `unstructured-wait`; groups nest as arrays with `value: null` last; a type with no status vocabulary is open and noted; the departures from `RFC-48`'s sketch are named; `export` carries the roadmap only when opted in. **Why:** maintainer decision, 2026-11-05, on the review of this record. | **substantive** |
