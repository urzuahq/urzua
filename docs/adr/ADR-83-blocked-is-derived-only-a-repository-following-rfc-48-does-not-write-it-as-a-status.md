---
Stable-Id: 01M6N54MXZ93MM5TD65PXP5HQ3
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-48, ADR-76
---
# 83 — Blocked is derived only: a repository following RFC-48 does not write it as a status

## Context

`RFC-48` makes `blocked` a derived value: a record is blocked when a record it depends on has not
succeeded, computed from `Depends-on`, never written. `ADR-72` left the derived value out of the status
declaration ("is not part of this decision") and `ADR-76` built the sequencing graph while saying
`RFC-48`'s other parts "are not decided here". `MILE-144` is that remaining part, and #313 built it
without a decision record. Its review blocked it for two reasons that matter here.

- Several readings a consumer will see were chosen in code, not decided: what counts as a dependency,
  what counts as finished, and which written statuses contradict being blocked.
- #313 kept `Blocked` as a written status that the rule demanded in both directions. `RFC-48` says the
  opposite for this repository: "This repository drops `Blocked` from the written vocabulary instead,
  because the engine computes it", and accepting the RFC includes amending `SPEC-6` to remove it.

On 2026-11-05 the maintainer decided that `Blocked` is **derived only**, per `RFC-48`. A repository that
follows `RFC-48` stops writing `Blocked` as a status. The wait is computed from `Depends-on` and surfaced
by `urzua check` and the roadmap.

Evidence of the cost of a written value: four milestones in this repository (`MILE-13`, `MILE-21`, `MILE-23`, `MILE-30`) wrote `Blocked` for a wait
that is a fact or a decision, not a record, so the written value had no `Depends-on` to agree with. A
written status that a rule must keep in step with a computed one is two sources for one fact, which
`ADR-64` clause 4 reduces to one.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep a written `Blocked` and require it both ways (the shape of #313) | Readable in a file without the tool | Two sources for one fact; contradicts `RFC-48`; a record that waits on a fact has no dependency to agree with |
| B. Derived only for this repository; the engine stays config-driven and a type may still opt in to a written blocked status (chosen) | One source; matches `RFC-48`; other adopters keep their vocabulary | Two readings in the engine, one a repository declares and one it omits |
| C. The engine refuses any written blocked status | One reading | Hardcodes a policy for every adopter; breaks `ADR-53` (every rule is declared, opt-in) |
| D. Count `Done` through `implementable` | No new key | `implementable` means "code may cite it" (`MILE-133`); a type that omits `Done` from it silently skips the check |
| E. Declare the statuses that claim progress as their own list (chosen for D's question) | Explicit; independent of citation | One more optional key |

## Decision

In the context of a derived `blocked` value that `RFC-48` accepts and no record yet decides, facing a
built version that kept a written status the RFC drops, we decided that **`blocked` is derived only
in a repository that follows `RFC-48`, that this repository declares no written blocked status, and that
the engine reads the derived value as follows**, to achieve one source for the wait and a check that a
record's written status does not claim progress it cannot have, accepting that a reader of a file alone
no longer sees "Blocked" and must run `urzua check` or `urzua roadmap`.

**The derived reading** (`sequence.status-agreement`, opt-in):

- A record is blocked when any record it depends on is not in a status its own type declares `success`.
  A dependency in a `failure` status, one with no `Status`, and one whose type declares no `success`
  therefore blocks. (A dependency in `failure` also draws `sequence.failed-dependency`, when built, for
  re-planning; this rule only says the dependent is not unblocked.)
- The dependency set is the sequencing pairs the repository declares in `roadmap.pairs` (the roadmap
  command's declaration, `MILE-182`), each an `acyclic` `relation_pairs` entry, read in either field, so
  a written `Blocks` counts. Check and the roadmap read the same declared set, so blocked in one can
  never disagree with blocked in the other. An `acyclic` pair that is not named there is not a
  dependency: a containment pair (parent and children) declared `acyclic` for cycle detection does not
  mark a child blocked by an open parent. Every pair named there is a dependency, whatever its
  reciprocity.
- A record that states a status claiming progress while it is blocked is reported. The statuses that
  claim progress are declared (below). `Planned`, and any status that does not claim progress, is
  correct for a blocked record and is not reported: a `Planned` record with an open dependency is
  simply derived-blocked. Moving a `Blocked` record to `Planned` is the migration `RFC-48` prescribes.
- A blocked value that cannot be computed is not guessed. A record with a claim on a record whose header
  cannot be read, or whose own header cannot be read while another record claims it through the inverse
  field, is counted unreadable, not reported as unblocked. A record with a `Depends-on` value that
  resolves to nothing is `sequence.unresolved-blocker`'s; this rule reports nothing further about it,
  since fixing the reference may change the answer.
- The suggested action (`ADR-73`) is `edit-field` `set` on `Status`. It carries a value only when the
  type declares exactly one written blocked status and the written status can transition to it; it is
  omitted for a record in a `success` status, whose transitions do not lead back to a waiting state.

**Whether `Done` counts.** `statuses.progress` is a new optional list: the statuses that claim work has
advanced, and so contradict being blocked. It must be a subset of `values` and disjoint from `failure`
and from `blocked`; it may include a `success` status. This repository declares `InProgress` and `Done`.
`Done` counts because a record cannot finish before its dependencies do (`RFC-48`). `implementable` is
not overloaded for this; it keeps its meaning (`MILE-133`).

**The engine stays config-driven.** A type may still declare `statuses.blocked`, the written statuses
that mean waiting, to opt in to the two-way check: a declared blocked status while nothing blocks the
record is reported, and a blocked record that states a `progress` status is reported. It must be a subset
of `values` and disjoint from `success`, `failure` and `progress`; a `progress` and `blocked` overlap is a
load error. A type that declares neither key is not examined. Enabling the rule while no type declares
either key, or `roadmap.pairs` declares no pair, is a config load error. A written status that a type
does not declare is not this rule's: it is unsupported under the status vocabulary, and urzua's own
config declares no `Blocked`. urzua's own repository follows `RFC-48`: it
declares `progress` and no `blocked`, so a written `Blocked` is not a declared status at all.

**The `SPEC-6` amendment.** `Blocked` is removed from the milestone `Status` value set. The set becomes
`Planned | InProgress | Done | WontDo`. A milestone whose written status was `Blocked` is moved to the
status that matches its real progress, with the wait kept in `Depends-on` (a record) and `Blocked-on`
(the explanation). A wait on a fact that is not yet a record stays in `Blocked-on` until the `condition`
type exists (`RFC-48`).

## Reversibility

Moderate. The engine change is additive and opt-in: both keys are optional and a repository that declares
neither is unchanged. Re-adding a written `Blocked` to this repository is a config edit plus migrating
records back, which is mechanical. What is harder to walk back is the `Blocked` removal from `SPEC-6`
once adopters of the milestone shape have migrated, and the `progress` key's name once it ships.

## Consequences

- `SPEC-3` gains `statuses.progress`; `statuses.blocked` is documented as the opt-in written reading.
  `SPEC-2` gains the rule's findings. Each gets a revision row. `SPEC-6` loses `Blocked`.
- `MILE-144` is rebuilt to this decision: the reading above, the load checks, and tests for each.
- This repository's milestone type drops `Blocked` from `values` and `transitions`. The milestones that
  wrote it are moved and keep the wait in `Blocked-on`/`Depends-on`.
- A wait on a fact is invisible to `urzua roadmap` until the `condition` type is built, so such a
  milestone may appear ready while waiting. That cost is not new; it was hidden behind the written
  `Blocked`. A tracked milestone for the `condition` type and the migration is wanted.
- The rule is inert until `roadmap.pairs` is declared, which ties it to the roadmap command's decision
  and sequencing: it should be accepted after, or together with, that record.

## Amendment (2026-11-07): a record held below its items only by a dependency is not out of order

*Status of this amendment: accepted by the maintainer on 2026-11-07. `Status` stays `Accepted` and the Decision text above is unchanged, except where it says what `sequence.status-agreement` reports.*

With `ADR-96` on, a record whose Done means items all resolve computes `InProgress` while a dependency is unfinished (the cap of the `ADR-96` amendment of the same date). Reporting that `InProgress` as a progress claim by a blocked record would leave no status the record can state. So `sequence.status-agreement` skips a blocked record whose status comes from that cap: its status is the type's `delivery.items.partial_status`, every item resolves, and an unfinished dependency is the only thing holding it below the type's `success` status. It emits an info notice, `all items proven ahead of dependency X`, naming the open dependency, and reports no error. The derived blocked value is unchanged otherwise: a record genuinely in progress out of order (an item unresolved), and a capped record that states `Done`, are still reported, and the type's `progress` list is unchanged. The rule reads the same blocked set as the cap, so the two cannot disagree. A configuration that declares no `delivery.items` is unchanged.

## References

- `RFC-48` -- the proposal this builds; its "Status: Blocked is computed this way" section.
- `ADR-76` -- the sequencing relation; `ADR-72` -- statuses and transitions; `ADR-73` -- suggested actions.
- `ADR-64` -- clause 4, a derived value is computed; `ADR-53` -- every rule is declared and opt-in.
- `MILE-144` -- the milestone this decision unblocks; `MILE-133` -- what `implementable` means;
  `MILE-182` -- the roadmap that surfaces the derived value.
- `SPEC-6` -- amended; `SPEC-2`, `SPEC-3` -- document the rule and the keys.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer decision of 2026-11-05 that `Blocked` is derived only, per `RFC-48`; #313's review found no decision record for the derived readings and a departure from the RFC. The maintainer approves before it is Accepted. | **substantive** |
> | 2026-11-05 | Amended while Proposed: the dependency set is the pairs `roadmap.pairs` declares, not every `acyclic` pair, so a containment pair never blocks and check agrees with the roadmap; the containment warning is dropped; `Planned` while blocked is not reported. **Why:** coordinator alignment with the roadmap decision. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05 (blocked is derived only, per RFC-48); accepted together with ADR-82. | **substantive** |
> | 2026-11-07 | Amendment added, accepted by the maintainer on 2026-11-07: `sequence.status-agreement` skips a blocked record whose `partial_status` comes from the `ADR-96` dependency cap (every item resolves, only an unfinished dependency holds it back) and emits an info notice, `all items proven ahead of dependency X`; a record genuinely in progress out of order is still flagged. **Why:** maintainer decision of 2026-11-07, so the two status rules can agree on a capped record without removing `InProgress` from `progress`, which would turn off out-of-order detection for every `InProgress` milestone. The Decision text is unchanged. | **substantive** |
