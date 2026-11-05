---
Stable-Id: 01M6N33XTAZBF07CBH3RPDB75W
Status: Accepted
Accepted-on: 2026-11-05
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
| Edge direction | Dependent to blocker vs. blocker to dependent | Edges run **blocker to dependent**: `A Depends-on B` is the edge `B -> A`. "Blocker first", `blocks`, and every walk in the critical-path tie-break follow that direction |
| Wave | Longest-path layer vs. earliest-start | Wave of an included open record = 0 with no open dependency, else 1 + the largest wave among its open dependencies; a dependency in `success` is ignored. Unique, so no tie |
| A dependency in a failure terminal | Treat as done vs. open vs. exclude | It does not unblock (`RFC-48`). The dependent and everything that transitively depends on it is excluded from waves, `ready` and the critical path, and listed with a `needs_replan` reason. The way out is to re-point the `Depends-on` (a `Superseded` target is re-pointed at its successor, the common case); the record then re-enters a wave |
| Critical path | Longest by count vs. weight; greedy vs. global; whole vs. included | No estimates (`RFC` non-goal), so count. Over included records only: the longest chain; among all longest chains, **the lexicographically smallest sequence of ids, blocker first** (not a greedy walk, which can take a shorter branch). Emitted once, for the one graph. A chain hidden behind an excluded record is visible in `needs_replan`, not in the path |
| Ordering | Unspecified vs. one rule | **Natural id order** everywhere: prefix compared as text, then the number compared as a number, then the whole id as text for an id with no number. Every list of ids, every map key set, `waves` inner arrays, `ready`, `blocks`, `blocked_by`, group members, `needs_replan` and `cycles` entries are ascending in it. `notes` are ordered by `kind` (the closed set's order below), then `record`, then `pair`, a null `pair` sorting first |
| Cycles | Abort vs. report and exclude | Each strongly connected component of size above one, or a record with an edge to itself, is listed once as an array of its members in ascending order, which starts at the smallest member. Members, and records that depend on them, are excluded and appear in `needs_replan` with reason `cycle` / `depends-on-cycle` |
| Relation matching nothing | Empty result vs. report | A declared pair no record carries a value for, an unresolved claim, and a claim on an unreadable record are `notes`. An empty result never stands for "nothing declared matched" |
| No declared pair | `null` vs. object | Always an object. No `roadmap.pairs` gives `records: {}`, empty lists and a `no-sequencing-pair` note |
| Grouping | Flat map vs. nested array | Declared `roadmap.group_by`, an ordered list of field names; the engine names none. Nested in `group_by` order as arrays; a record with no value for a field goes in a bucket with `value: null`, emitted last. A name no type declares is a config load error (`ADR-64` clause 5); a field no open record carries is a note |
| Shape | Ids only vs. a per-record map | A `records` map keyed by id carries the per-record data, so a page needs no join for the common fields. `records` holds every open record, `needs_replan` ones included (with `wave: null`); `groups` holds the same set, so an excluded record is still found under its group |

## Decision

*In the context of `RFC-48`'s accepted but unspecified `urzua roadmap`, facing a public output and
readings that change what a consumer sees, we decided that the roadmap is one graph over the
sequencing pairs named by `roadmap.pairs`, using the edges `sequence.cycle` checks (so a loop that
crosses declared pairs is found, by the roadmap; `check` still sees cycles per pair until a follow-up milestone adds a mode over `roadmap.pairs`); that wave, ready, critical path, ordering and cycle listing are as
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
    "waves": [["MILE-5"], ["MILE-7", "MILE-8"], ["MILE-12"]],
    "ready": ["MILE-5"],
    "critical_path": ["MILE-5", "MILE-7", "MILE-12"],
    "records": {
      "MILE-5": {"type": "milestone", "status": "Planned", "wave": 0, "ready": true,
                 "blocked": false, "blocked_by": [], "blocks": ["MILE-7", "MILE-8", "MILE-12"]},
      "MILE-7": {"type": "milestone", "status": "Planned", "wave": 1, "ready": false,
                 "blocked": true, "blocked_by": ["MILE-5"], "blocks": ["MILE-12"]},
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

- `blocks` and `blocked_by` are transitive and direct respectively (in the example `MILE-12` depends on `MILE-7`, which depends on `MILE-5`, so `MILE-5`'s `blocks` lists `MILE-12` although `MILE-12`'s `blocked_by` is `MILE-7` only): `blocked_by` the direct open or
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

- `MILE-182` depends on this ADR and is blocked until it is Accepted. `blocked` is derived only, so the roadmap and the derived value show that with no written status, and accepting this ADR needs no edit to `MILE-182`'s `Status`. This ADR should be accepted together with, or after, the decision record that makes `blocked` derived only.
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

## Amendment (2026-11-05): a repository may declare which record types are work, and the roadmap orders only those

*Status of this amendment: proposed, pending maintainer acceptance. Drafted 2026-11-05 at the maintainer's request (decision of 2026-11-05: amend this record with an opt-in setting that declares which types are work). `Status` stays `Accepted` and the Decision text above is unchanged; nothing below is built until the maintainer accepts it (`MILE-196`).*

The Options table defines "open" as a status in neither `success` nor `failure`, and ready as open and not
derived-blocked. That treats every record type as work. A repository can declare a type for a fact nobody
builds (`SPEC-23`, `MILE-187`): an `Open` record is a wait that is met or dropped, never started, finished
or ordered. Measured on this repository's corpus at `0fa6d374`: `urzua roadmap` puts `COND-1` to `COND-4`
in wave 0 and in `ready`, and `COND-1` on the critical path (`COND-1`, `MILE-13`), as if they were buildable
(`rules/roadmap.rs:315-319` takes every record whose standing is open into the open set; `:466` makes
a record ready when it is included and unblocked). No configuration removes them: `roadmap` accepts only
`pairs` and `group_by` (`config.rs:85-92`, `deny_unknown_fields` at `:84`), and `group_by` only groups
(`rules/roadmap.rs:533-540`). Found by the independent review of `#349`.

**1. The setting.** `roadmap.work_types` is an optional list of record type names, each a key of
`record_types`. It says which types are work: the records the roadmap orders. It names no type itself; the
engine knows no type name (`ADR-64`), and `condition` is only this repository's example of a type a
repository may leave out. Any type that models a fact, a signal, a decision awaited or an external event
is a candidate.

**2. Absent means no change.** With `work_types` absent, every type is work and the roadmap is
byte-identical to today's: same records, waves, ready set, critical path, groups, notes and keys, and no
`waiting_on` key (item 5). A repository that has not opted in sees nothing.

**3. A declared non-work record still gates its dependents, and is never ordered itself.** With the key
declared, a record whose type is not listed is non-work. It stays a node of the one graph. What it means
for its dependents is unchanged: a dependent of an `Open` non-work record is blocked, and out of `ready`
(it is derived-blocked, `ADR-83`); a dependent of a `Met` (success) one is released, as of any record in
a `success` status; a dependent of a `Dropped` (failure) one goes to `needs_replan` with `failed-dependency`
and `via` naming the non-work record. What changes is that a non-work record is not itself a subject of the
output:

- not in `waves`, `ready` or `critical_path`;
- not a key of `records`, which holds the work records it did before, and not in `groups`;
- not counted by `nothing-open` (that note means no work record is open) and not a subject of
  `unstructured-wait` (a fact does not wait to be ordered);
- not in `needs_replan` as a record (its dependents are, where item 3 says so). Exclusion still propagates through it,
  so a work record behind a non-work record that itself waits on a failed work record is
  `depends-on-excluded`, with `via` the non-work record;
- `no-status-vocabulary` is still raised for a non-work type with no status vocabulary: it is still read as
  open, and a wait that is never read as finished blocks forever, which is worth saying.

**4. Waves and the critical path count work records, with edges contracted through non-work ones.** For waves
and the critical path, dependency edges contract through chains of non-work records: a work record depends
transitively on every work record that any non-work record on its path depends on. Only work records are
ever counted, but a non-work record in between does not cut the ordering. A work record's wave is 0 with no
open work dependency in the contracted graph, else 1 + the largest wave among them. A non-work record's own
status still gates, as item 3 says: `Open` blocks its dependents, `Met` releases them, `Dropped` sends them to
`needs_replan`. Contraction orders work; it never releases a gate.

Worked example. `W` (work, `Open`) <- `C` (non-work, `Met`) <- `R` (work), that is `C` depends on `W` and `R`
depends on `C`. `R` depends transitively on `W` through `C`, so `W` is wave 0 and `R` is wave 1, not wave 0.
Without contraction both would be wave 0, and a wave would not be strictly after the work `blocks` says it
waits for. With `C` `Open`, `R` is also `blocked` (blocked by `C`) and not `ready`; with `C` `Dropped`, `R` is
`needs_replan` with `failed-dependency` via `C`. The critical path follows the same contracted edges: `W`, `R`
is a chain of length 2.

The consequence is stated, not hidden: the Options table's "ready is wave 0 by construction" holds only when
every dependency is work. With the key declared, `ready` is a subset of wave 0: a work record whose only open
dependency is a non-work record is in wave 0 and not ready, and reads `blocked: true` with the non-work id in
`blocked_by` (`blocked_by` is unchanged: the direct dependencies not in `success`, of any type). `SPEC-2` will
say so. `blocks` lists work records only, but reaches them through non-work ones: if `MILE-13` depends on
`COND-1` and `MILE-30` depends on `MILE-13`, `MILE-13`'s `blocks` lists `MILE-30`. The tie-break of the
critical path is unchanged.

**5. Where non-work records are shown: a top-level `waiting_on` array (recommended).** Hiding them
would hide the very information that made them worth declaring. Three placements were considered.

| Placement | Pros | Cons |
|---|---|---|
| A. `waiting_on`, a top-level array of the non-work records that are not in `success`, each `{record, type, status, blocks}` in natural id order, `blocks` the work records that transitively depend on it (**recommended**) | One list answers "what is the work waiting on, and what does each wait gate"; named for the role, not for any type; `Met` records, which gate nothing, drop out; additive under `schema` 1 | A consumer joins `blocked_by` on a work record to `waiting_on` by id for the per-record view |
| B. `conditions`, the same list under a type-flavoured name | Reads well in this repository | Names a type the engine does not know; wrong for a repository whose non-work type is called something else |
| C. A `waiting_on` list on each work record, inside `records` | No join | Repeats one wait on every dependent; says nothing of a non-work record that blocks nothing yet; changes every entry's shape |

Recommended: A. The key is present **only when `work_types` is declared** (empty list when every non-work
record is `Met`), so a repository without the setting keeps its output unchanged (item 2). Adding a key is
non-breaking and `schema` stays 1 (Decision, notes paragraph). A `Dropped` non-work record is listed:
it gates dependents into `needs_replan` and someone must re-point them. `export --format json` carries
`waiting_on` as part of the `roadmap` object.

**6. Cycles, `check` and the derived blocked value are unaffected.**

- `cycles` still lists a loop's members whatever their type: a loop is a fact about the graph, not about
  what is work, and a non-work record on a loop is a member (`rules/roadmap.rs:324-339` reads the whole
  merged graph before any type is consulted). A work record behind a cycle member is `depends-on-cycle`
  as before.
- `sequence.cycle` (`MILE-192`) reads every `acyclic` pair's slots with no type filter
  (`rules/sequence.rs:26`) and merges the pairs `roadmap.pairs` names (`:27-45`, `merged_graph` at `:197-205`). It never reads
  `work_types`, so the cross-pair reading of `MILE-192` finds a loop through a non-work record exactly as
  today, and `check` and the roadmap still agree on which loops exist.
- `sequence.status-agreement` (`ADR-83`) decides blocked from the dependency's own `success` set
  (`rules/sequence.rs:576-582`, `:594`) and chooses which records to examine by the dependent's declared
  `progress`/`blocked` statuses (`:533-538`). It never reads `work_types`, so a work record with an `Open`
  non-work dependency is derived-blocked in `check` and in the roadmap alike, and a non-work dependent is
  examined or not by its own type's declaration as before. `ready` implies not `blocked`
  in both readings.

**7. Validation, at config load.** In `validate_roadmap` (`config.rs:2767`, called at `:1841`, reported as
`roadmap: ...` through `ConfigError::InvalidRoadmap`, `:1267-1268`), alongside `pairs` and `group_by` and under
`ADR-64` clause 5 (a declaration that can never apply is refused, not read as a clean roadmap):
a name that is not a `record_types` key is a load error naming it (the case the maintainer named: an unknown
type name); a name listed twice is a load error; a blank name or one with surrounding whitespace is a load
error; and an empty list is a load error, since a declaration that nothing is work orders nothing. A
`work_types` with no `roadmap.pairs` is accepted and inert, and the existing `no-sequencing-pair` note
(`rules/roadmap.rs:211`) already says there is no roadmap. A binary without this setting rejects the key
(`deny_unknown_fields`, `config.rs:84`), so a repository that declares it cannot be silently misread by an
older `urzua`.

**8. Reversibility.** The key name and the `waiting_on` shape are the parts to settle; both are additive
and a rename is a config migration (`ADR-77`). Narrowing the work set later moves records out of the output,
the direction this Decision's open-set row warns about, which is why it is opt-in per repository and
changes nothing for one that does not declare it.

**9. Follow-through.** `SPEC-2` (the roadmap output: `waiting_on`, and the ready-versus-wave-0 sentence),
`SPEC-3` (the `roadmap.work_types` key and its load errors) and `SPEC-23` ("How a condition is read")
gain revision rows when this is built (`MILE-196`). This repository then declares `work_types` in
`.urzua/config.yaml`, so `COND-1` to `COND-4` leave the ready set and `COND-1` leaves the critical path.

## References

`RFC-48`, `ADR-76`, `ADR-64`, `ADR-73`, `ADR-77`, `MILE-182`, `MILE-144`, `MILE-115`, `SPEC-2`, `SPEC-3`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** `MILE-182` builds `RFC-48`'s `urzua roadmap`, no accepted record decides its waves, ready set, critical path or output shape, and an agent may not accept it; the maintainer decides. | **substantive** |
> | 2026-11-05 | Amended while Proposed after design review: one graph over the pairs `roadmap.pairs` declares (a cross-pair loop is detected); ready means open and not derived-blocked; natural id ordering, the critical-path tie-break and the cycle listing fixed; `needs_replan` reasons and `via` defined; output always an object with `schema`, a per-record map and structured notes from a closed set including `unstructured-wait`; groups nest as arrays with `value: null` last; a type with no status vocabulary is open and noted; the departures from `RFC-48`'s sketch are named; `export` carries the roadmap only when opted in. **Why:** maintainer decision, 2026-11-05, on the review of this record. | **substantive** |
> | 2026-11-05 | Re-check amendments: edge direction fixed (blocker to dependent); `records` and `groups` include `needs_replan` records; a null `pair` sorts first in `notes`; a transitive `blocks` example; Consequences no longer says acceptance moves `MILE-182`'s status, since `blocked` is derived only; a follow-up milestone adds a `sequence.cycle` mode over `roadmap.pairs`. **Why:** design re-check of 2026-11-05. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05, after an independent review and re-check found no remaining blockers; accepted together with ADR-83. | **substantive** |
> | 2026-11-05 | Amendment added, proposed and pending maintainer acceptance: an opt-in `roadmap.work_types` declares which record types are work; a non-work record still blocks its dependents but never appears in waves, `ready`, the critical path, `records` or `groups`, and is listed in a top-level `waiting_on`; `check` and the derived blocked value are unaffected; an unknown type name is a load error. The Decision text and `Status` are unchanged. **Why:** `#349` review: open `condition` records sit in wave 0 and the ready set and no configuration removes them; maintainer decision, 2026-11-05, to amend. | **substantive** |
> | 2026-11-05 | Amendment revised while proposed after independent review: waves and the critical path contract dependency edges through chains of non-work records, so a work record behind a non-work record that is behind open work is in a later wave; a non-work record's own status still gates; worked example added; `ready` is a subset of wave 0 when the key is declared, and `SPEC-2` will say so. **Why:** review of `#350` found waves disagreed with `blocks`; the amendment is still proposed and `Status` stays `Accepted`. | **substantive** |
