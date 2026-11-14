---
Stable-Id: 01M7BS20RZ12YZE0M0K7RAY6WK
Status: Accepted
Accepted-on: 2026-11-14
Date: 2026-11-14
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 101 — Urzua's mission and the principles every decision is judged against

## Context

Urzua has a founding claim (`SPEC-1`: one config-driven implementation instead of every team hand-writing its own linter) and, by
now, 100 decisions that each follow from it. What no record states in one place is the mission those decisions serve and the
principles a new proposal is measured against. They are scattered: the README says what the tool is, `SPEC-1` states the cross-cutting
rules and the exit bars, and each principle was established by a different ADR. A reviewer or an agent judging a proposal has to
reconstruct them, and a proposal can pass every rule while quietly working against the purpose.

The situation that makes this urgent is agents. Agents now write code and also the records that justify it: decisions, specs, a
backlog, bug reports. A record an agent wrote is prose that claims things about the code, and nothing makes the claim true,
current or accepted by the right person. The README states the gap directly: "Your agent just wrote a record of a decision it made.
Is it structured data, or prose that happens to have a `Status:` line at the top?"

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave the mission in the README and `SPEC-1` | Nothing to write | The principles stay scattered across a README, a parent spec and many ADRs; no record a review can cite; no rule for a conflict |
| B. Record the mission and principles as a spec | Edited in place as the product evolves | A spec is current truth, so a change to a principle would be an edit rather than a decision; the thing to preserve is the decision to hold these lines |
| C. Record them as an ADR that cites the ADRs and specs establishing each principle | One citable decision; each principle keeps its own record; an amendment is the visible way to change one | A summary that can drift from the records it cites, so it cites them and adds no rule of its own |

## Decision

*In the context of agents writing both code and the records that justify it, facing mission and principles that no record states in
one place, we decided that Urzua's mission and the principles below are the standard every later decision, review and ADR is judged
against, to achieve one citable reference that a proposal must either satisfy or amend first, accepting that this ADR must be
amended whenever a principle changes.*

**Mission.** Urzua is the verifiable layer between agent-authored change and the decisions behind it. It makes the records that
justify a change structured data with declared types, lifecycles and relationships. It checks with evidence that each record's
claims about the code are still true, and it makes sure only the right person can accept a decision. It does this through one
machine-first output contract that agents and CI branch on. It checks the records a change traces to, and never judges the change
itself (`SPEC-1`, the permanent content-scope ceiling).

**What Urzua is for.**

1. **Records are data, not prose.** Record types are declared in configuration and the engine validates against the declaration; it
   is a general engine, not an ADR tool. This repository runs its own milestones and bugs on it as ordinary configuration with no
   change to the engine (`README`; `ADR-34`, `ADR-53`, `ADR-64`).
2. **Claims are backed by evidence.** `Done`, `Implemented` and `Verified` point at pinned code and tests, drift between a claim and
   the code is detected, and renewing an expired claim needs an honest reason (`ADR-84`, `ADR-96`, `ADR-97`).
3. **People own decisions.** Agents draft; only a maintainer moves a decision to `Accepted`. This is enforced in CI by the decider
   gate and in the agent harness (`ADR-89`; `ADR-27` for `urzua new` filling in the template and not the decision).
4. **Machine-first.** Every command returns a status and typed findings as JSON on stdout, in one report shape, so tools compose
   without re-interpreting prose: CI gates, the MCP server and harness hooks (`ADR-23`, `ADR-46`, `ADR-64`, `ADR-99`).
5. **One engine instead of every team's reinvented validator.** The bar is replacement, not coexistence: it is proven by replacing a
   real adopter's hand-written linter, which is then deleted (`SPEC-1`, Exit bar; `MILE-102`).

**Standing principles.**

- **Opt-in configuration.** A behaviour that is new is declared in configuration, and a configuration that does not declare it
  behaves as it did before (`ADR-53`, `ADR-92`, `ADR-100`).
- **No silent no-ops.** A check that finds nothing must be distinguishable from a check that ran on nothing, and a declared input
  that matches nothing is reported (`SPEC-1`, "Non-negotiable: no silent no-op"; `ADR-55`).
- **Nothing is `Done` without evidence.** A record's state follows its declared evidence field and is computed, not asserted
  (`ADR-84`, `ADR-96`).
- **Strict on its own repository, with zero standing warnings.** This repository runs `strict` with `fail_on: warning`, so a finding
  is fixed or waived with a reason and never left standing (`SPEC-1`, Exit bar; `ADR-85`, `ADR-88`).

**Phases.** The phases are the order in which the mission is delivered, and `SPEC-1`'s Exit bar defines the first two. Phase 0 is done
when Urzua governs its own repository in CI at parity with a real adopter environment and with zero standing warnings. Phase 1 is done
when it replaces an adopter's linter (`MILE-102`), runs green in two codebases with different primary languages (`MILE-99`,
`MILE-100`), reproduces every documented bug (`MILE-101`) and `urzua new` is faster than hand-authoring (`MILE-103`). Phase 2 is deeper
agent integration (`MILE-13`, `MILE-17`, `MILE-29`). Phase 3 is attestation, non-git identity and backfill (`MILE-19`, `MILE-20`,
`MILE-31`). The milestones, not this ADR, carry the detail and the current state.

## Reversibility

Easy to walk back, because it adds no code and no rule: it records a standard and cites the records that carry the behaviour. A
principle can be changed by an amendment to this ADR, which is the intended friction. The cost of getting it wrong is a reviewer
holding a proposal to a principle that has since moved, which the amendment requirement is there to prevent.

## Consequences

- Reviews and future ADRs are judged against this one. A proposal that conflicts with the mission or a standing principle needs an
  amendment to this ADR first, so the conflict is a recorded decision and not a drift.
- A principle now has one citable place, and the records that established it stay the authority for its detail; this ADR repeats
  none of their rules.
- This ADR is a summary of other records, so it can go stale when they change. It cites each by number so that a change there is
  found by a search for the citation, and an amendment here is the cost of a principle changing.
- Phase 2 and Phase 3 are named only as pointers; deciding their scope remains the job of their milestones and any ADR they need.

## References

`README`; `SPEC-1` (purpose, no silent no-op, exit bars); `ADR-23`, `ADR-27`, `ADR-34`, `ADR-46`, `ADR-53`, `ADR-55`, `ADR-64`,
`ADR-84`, `ADR-85`, `ADR-88`, `ADR-89`, `ADR-92`, `ADR-96`, `ADR-97`, `ADR-99`, `ADR-100`; `MILE-99`, `MILE-100`, `MILE-101`, `MILE-102`,
`MILE-103`, `MILE-13`, `MILE-17`, `MILE-19`, `MILE-20`, `MILE-29`, `MILE-31`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-14 | Drafted as Proposed. **Why:** Approved by the maintainer in conversation on 2026-11-14; Status moves to Accepted in a follow-up the maintainer approves. | **substantive** |
> | 2026-11-14 | Status changed from Proposed to Accepted. **Why:** Accepted by the maintainer on 2026-11-14. | **substantive** |
