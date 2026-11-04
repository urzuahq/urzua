---
Stable-Id: 01M6DFQKHEX940BY1Y6ZHGZJHX
Status: Accepted
Date: 2026-11-02
Author: beauwilliams
---
# 48 — The roadmap is computed from records

## Summary

Make the order of work a computed view over records, the way `audit` is a computed view over
findings. Records already carry the inputs (`Status`, `Phase`, `Track`, `Parent`, `Blocked-on`,
`Implements`); the engine never turns them into an order. This proposes three things: a declared
sequencing relation with cycle detection (`MILE-27`), an `urzua roadmap` command that derives waves,
the ready set and the critical path from that relation, and a small set of rules that keep the
relation honest. All three are engine features driven by config, usable by any adopter; this
repository is the first user.

## Motivation

`ADR-34` deferred both pieces with explicit triggers. The sequencing relation waited for "a real case
needs it"; the generated roadmap waited for "enough milestones for a generated view to earn its cost".
Both triggers have now fired:

- **The real case.** Planning Phase 0's remaining work needed an order that `Phase`/`Track` cannot
  express. Seven parallel lanes all edit `rules.rs`, so they wait on `MILE-131`; the switchover
  (`MILE-102`) waits on `MILE-120` and `MILE-98`; six decisions gate whole lanes. That order was
  worked out by reading records and written into a hand-built HTML page. It is already the drift
  `ADR-34` warned about: the page is a second document that nothing checks against the records.
- **The scale.** There are 134 milestones. `urzua check docs/milestones/` lists findings; it does not
  say what can start now.
- **The data is there but thin.** 124 records carry `Blocked-on:`. 96 of them hold the `—`
  placeholder, and only 23 cite a record ID; the rest are prose ("a real external adopter asking for
  it"). `urzua graph` does emit `Blocked-on` and `Parent` edges, but as `narrative` edges with no
  direction semantics and no cycle check, so nothing can be ordered from them.

Separately, the pattern of epics, sub-issues, dependency links and board stages computed from state is
common practice for engineering roadmaps kept in an issue tracker. The gap that practice leaves is
that the plan lives outside the decision records it implements, so a milestone can reach `Done`
while the board still shows it blocked, and the reverse. Computing the roadmap from the records closes
that gap by construction.

## Proposal

### 1. A declared sequencing relation (`MILE-27`)

A new `Depends-on` field carries sequencing: a comma-separated list of record IDs, "this record cannot
finish until those do". It is a pointer field, so every value must resolve.

`Blocked-on` stays, as a separate free-text narrative field. The two answer different questions:
`Depends-on` is a typed dependency the engine orders and derives `blocked` from; `Blocked-on` is a
human note on what is in the way, which no rule orders on. A blocker the engine should act on is
written as `Depends-on` (to a record or a condition record); `Blocked-on` explains it. This is the
stated-reason exception `ADR-64` allows for a prose header field. The `—` placeholders are dropped,
since an absent optional field says the same thing.

`Depends-on` is declared as a pair through `RFC-47`'s relation declarations:

```yaml
relation_pairs:
  - name: sequencing
    forward: "Depends-on"
    inverse: "Blocks"
    reciprocity: none        # one direction is written; the inverse is computed
    cross_type: true
    acyclic: true            # new: the engine checks this pair for cycles
```

Each direction is a unidirectional relation, declared once and named twice. An adopter who wants both
sides written in the files sets `reciprocity: required` and gets the existing reciprocity check; this
repository writes `Depends-on` only and reads `Blocks` from `urzua roadmap`. `acyclic` is the one new
key, and it applies to any pair, not only sequencing.

`Parent` is declared the same way as a containment pair (`Parent` / `Children`), which is how a
milestone acts as an epic over child milestones and bugs.

### 2. `urzua roadmap`

A read-only command that computes, for a scope (default: every type with a sequencing relation):

- **Waves**: topological layers over open records; wave 0 is everything whose blockers are all in a
  success terminal state.
- **Ready set**: open records whose blockers are all in a success terminal state. A blocker in a
  failure terminal state keeps its dependent out of the ready set (`sequence.failed-dependency`).
- **Critical path**: the longest chain of open blockers.
- **Per record**: what it blocks, transitively.
- **Grouping** by `Phase` then `Track` (fields named in config, not hard-coded).

Output is JSON by default, and part of `export --format json` (`MILE-115`), so a dashboard or
roadmap page is generated from it rather than hand-written. Proposed shape: a top-level `roadmap`
object with `waves` (array of arrays of record IDs), `ready` (record IDs), `critical_path` (record
IDs in order) and `groups` (keyed by the configured grouping fields), and on each exported record a
`derived` object holding its derived values (`blocked`, `blocks`, `children`). The exact field names
are fixed in the SPEC that implements this RFC. Like `audit`, nothing is stored.

### 3. Rules

- `sequence.cycle`: a cycle in a sequencing relation. Error.
- `sequence.terminal-parent-open-child`: a parent in a terminal status with a child that is not.
- `sequence.phase-order`: a record blocked on a record in a later phase.
- `sequence.failed-dependency`: a `Depends-on` target reached a failure terminal state; the
  dependent needs re-planning.
- `sequence.unresolved-blocker`: a `Depends-on` value that does not resolve to a record. A fact that
  is not yet a record becomes a condition (below).
- The existing `narrative-field.stale` / `pointer.target-status` behavior covers a blocker that has
  since reached a terminal status.

### 4. This repository's workflow

- Re-record the order the hand-built Phase 0 roadmap holds as `Depends-on` and `Parent` values, so
  the records carry it.
- Generate the roadmap page from `urzua roadmap` output, the same way the ledger page is generated
  from records, and retire the hand-built page.

## Conditions: dependencies on facts nobody builds

Some dependencies are not work. "An external adopter asks for AgDR round-tripping" or "RFC-9's Q2 is
answered" are facts that are observed, not built, and a milestone is the wrong shape for them. A
`condition` record type holds them, so a dependency on a fact is as structured as a dependency on work.

```yaml
---
Status: Open            # Open | Met | Dropped; a Met condition adds Met-by (evidence: a record ID, a commit, a link)
Review-by: 2027-02-01   # optional: when to re-ask whether the condition is still plausible
---
# 1 — An external adopter requests AgDR round-tripping

## Signal
One sentence on exactly what counts as met.
```

- **Configuration, not an engine type.** `condition` is declared in config (`dir: docs/conditions`,
  prefix `COND`, filenames `COND-NNNN-slug.md` per `ADR-36`) with a state machine built from
  `MILE-132`'s primitive: `Open -> Met`, `Open -> Dropped`, both terminal; `Met-by` required in `Met`.
  The engine has no special case for it. It is the first record type composed entirely from
  configuration, which makes it the test that the primitives are sufficient.
- **Terminal states split into success and failure.** A state machine declares each terminal state as
  `success` (Met, Done, Fixed, Accepted) or `failure` (Dropped, WontDo, WontFix, Rejected, and
  Superseded, because a dependency on a superseded record must be re-pointed at its successor). A
  `Depends-on` target in a success state unblocks its dependent. A target in a failure state does not
  silently unblock it: `sequence.failed-dependency` reports the dependent for re-planning. Rules
  refer to "success" or "failure" from the declaration instead of copying status lists, which removes
  the cause of `BUG-178`.
- **Review dates.** `condition.review-due` warns when an `Open` condition's `Review-by` has passed, so
  a condition such as "a real adopter asks" cannot block work indefinitely without anyone re-asking.
  The same date semantics as a waiver's `Expires`.
- **Humans flip the status.** Moving a condition to `Met` or `Dropped` is a human decision, like any
  Status. `MILE-135`'s later forge tool may propose `Met` from evidence it finds; a person accepts it.
- **Migration for this repository.** Prose blockers that name facts become conditions with a
  `Depends-on` from the waiting record: "a real external adopter asking for it", "real adopter demand
  for AgDR round-tripping", "real adoption data on how much the deprecated shapes are still in use".
  "A real case where Phase/Track tags aren't enough" becomes a condition created `Open`, with this RFC
  proposed as its `Met-by` evidence; a person moves it to `Met`. Prose blockers that name decisions ("a decision to actually build it", "RFC-9's
  own Q2") point at the RFC or a decide-milestone instead. `Blocked-on` keeps any explanatory note.

## How this composes in the engine

None of this is a milestone feature. Every piece above is configuration over five primitives: three
exist today, the state machine comes from `MILE-132`, and the derived value is new in this RFC, so an adopter opts in or out per type and per pair:

| Primitive | Today | Added here |
|---|---|---|
| Declared field | `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields` | `Depends-on` as a pointer field |
| Declared relation | `RFC-47` pairs: forward, inverse, reciprocity, cross-type | `acyclic` on a pair |
| Derived value | none: every value is read from a file | a value computed from relations, never written (`blocked`, `Blocks`, `Children`) |
| Rule | opt-in in `rules:`, with options | `sequence.*` and `condition.review-due` rules, each opt-in |
| State machine | none: status lists copied into rule options | declared per type (`MILE-132`), terminal states marked success or failure |

- **A derived value** is the one primitive this RFC adds. A type may declare values computed from its relations,
  for example `blocked: any Depends-on target not in a success terminal state`. Derived values appear in
  `export`, `graph` and `roadmap`, and rules can read them, but they are never written into a file.
- **`Status: Blocked` is computed this way.** The roadmap shows a record as blocked whenever its derived
  `blocked` value is true, whatever its written `Status`. An adopter who still writes `Status` by hand
  can opt into `sequence.status-agreement`, which reports a written `Blocked` with no open dependency,
  and an open dependency on a record whose written status says otherwise. This repository drops
  `Blocked` from the written vocabulary instead, because the engine computes it: accepting this RFC
  includes amending `SPEC-6`'s `Status` value set to remove `Blocked`, and each milestone whose
  written `Status` is `Blocked` is moved to `Planned`, with its blocker recorded as a
  `Depends-on` value.
- **`urzua roadmap` is a view over these primitives**: waves from any `acyclic` pair, grouping by any
  declared field. A repository that declares no acyclic pair has no roadmap and loses nothing else.

## Lifecycle stages are a workflow, not part of this proposal

Moving an epic through ordered stages (for example Define, Design, Build, Launch, Operate) is a
workflow an adopter composes from configuration, not an engine feature. The engine piece it needs is
an ordered value vocabulary with per-value required fields, which belongs to `MILE-132`; a stage
workflow is then configuration on top of it, adopted or not per repository.

## Open questions

1. Whether a derived value is declared per type in config or defined by a rule. Per type keeps it
   visible in one place; per rule keeps the config smaller.
2. Whether `Children` should be listed per record in `export` or only in `graph`.

## Non-goals

- Reading issue trackers or pull requests. The roadmap is computed from records in the repository.
  A later tool that reads open changes as evidence of progress is filed as `MILE-135`.
- Scheduling, estimates, dates or assignment.
- Storing the computed order anywhere, or a hand-edited board.

## References

- `ADR-34` -- deferred both pieces with the triggers this cites.
- `SPEC-6` -- the milestone schema, whose "deliberately not built" section names both.
- `MILE-27` -- the sequencing relation this builds.
- `RFC-47` -- relation declarations with direction; a prerequisite.
- `MILE-135` -- later: open changes as roadmap progress evidence.
- `MILE-115` -- `export --format json`, the data feed.
- `MILE-131`, `MILE-132` -- the rules split and status vocabulary this interacts with.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Drafted. **Why:** Planning Phase 0 needed an order the records could not compute, which is the trigger `ADR-34` set for both deferred pieces. | **substantive** |
> | 2026-11-02 | Sequencing moves to a new `Depends-on` pair (`Blocked-on` stays narrative); `Blocked` becomes a derived value; added the engine composition and lifecycle sections; forge evidence filed as `MILE-135`. **Why:** Author's answers to the first draft's open questions. | **substantive** |
> | 2026-11-02 | `Blocked-on` retired in favour of `Depends-on`; lifecycle stages moved out as a workflow over `MILE-132`. **Why:** Author: one field for waiting, and stages are workflow composition. | **substantive** |
> | 2026-11-02 | External conditions become records too, not body prose. **Why:** Author: structured in and out. | **substantive** |
> | 2026-11-02 | `Blocked-on` kept as a separate narrative field beside `Depends-on`. **Why:** Author: a dependency and a free-text note on what is in the way are different fields. | **substantive** |
> | 2026-11-02 | Added the `condition` record type, success and failure terminal states, review dates and the condition migration. **Why:** Author accepted the recommendations from the condition-type discussion. | **substantive** |
> | 2026-11-02 | Readiness and `blocked` count only success terminals; Superseded classed as failure; condition example and migration no longer pre-set `Met`; `SPEC-6` amendment for `Blocked` named; primitive count fixed; roadmap JSON shape proposed. **Why:** Review findings. | **substantive** |
> | 2026-11-04 | `Draft` -> `Accepted`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
