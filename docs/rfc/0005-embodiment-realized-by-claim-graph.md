# 0005 — Embodiment as a schema-level concept; `realized_by` as a claim graph

> Status: Accepted
> Date: 2026-07-30
> Author: (session author)
> Supersedes / Superseded-by: —

> **Split from RFC-0001, 2026-07-30.** This was originally §3, §3a, and §3b of
> `0001-unified-schema-and-layering.md` — the largest and most-corrected part of that RFC (two of
> its three inline "Correction" notes belonged to this section alone). Split so it's independently
> acceptable/rejectable from the core schema shape and the layering model. Content is unchanged
> from the original §3/§3a/§3b, per this repo's own rule against silent rewrites. RFC-0002
> previously cited this content as "RFC-0001 §3a"/"§3b" — updated to cite this RFC directly as
> part of the same split.

## Summary

Lift an Embodiment model (`Not started → Specified → Implemented → Verified`, plus `Drift
detected` and `Inactive`) into the core schema as the realization axis, orthogonal to `status` (the
decision axis). Computation is back-pointer-based, not inference-based. The back-pointer itself
lives on the record (`realized_by`), not embedded as a comment in application code, and is a small
claim graph (AND/OR composites over typed, confidence-ranked leaves) rather than a flat list —
because a multi-part decision's confidence must be capped by its weakest required part, and
evidence needs to be shareable across records rather than duplicated per-record.

## Motivation

See RFC-0001's Motivation for the broader schema-unification case. This section's specific
motivation is narrower and sharper: Embodiment is the single most valuable design idea this project
draws on, and getting its addressing/confidence model wrong produces two real, observed failure
classes — a back-pointer comment that proves a string was typed once but not that the code still
matches (stale-but-present drift), and a flat evidence list that can't express a multi-part
decision's weakest link or share evidence across records that describe the same underlying work
(the ADR-0072/0073/0074 case below).

## Proposal

### 1. Embodiment is a schema-level concept, not ADR-specific

An Embodiment model — `Not started → Specified → Implemented → Verified`, plus `Drift
detected` (alarm, reachable from any state) and `Inactive` (terminal, negative decisions) — is the
single most valuable design idea this project draws on. Propose lifting it into the core schema as
the *realization axis*, orthogonal to `status` (the *decision axis*), for any record type where "was
this actually built" is a meaningful question (ADR and Spec; not RFC — a pre-decision proposal
cannot itself be "implemented," only the ADR/Spec that eventually derives from it can).

Computation stays back-pointer-based, not inference-based — a deliberate, correct choice that should
not be second-guessed here: back-pointers are grep-able, auditable, and don't require an LLM or
heuristic to compute. Save inference (LLM-assisted "does this code actually match the ADR's
intent") for the *drift-detection alarm's investigation step*, not for the base computation of
whether a back-pointer exists at all.

### 2. The pointer lives on the record, not embedded in application code

The original assumption — the back-pointer is a comment embedded in the target, `Implements:
ADR-NNN` inside a source file or test — is wrong on maintainability grounds, and wrong on coverage
grounds; both were found the hard way running exactly this mechanism across independent codebases:

- **A comment only proves someone typed a string once.** It doesn't prove the code still does what
  the record claims — a stale-but-present `Implements:` comment passes a grep-based audit exactly
  as cleanly as an accurate one. A real drift case is precisely this shape:
  the comment (or the record) said one thing, the code did another, and the mechanical check
  couldn't tell the difference because it was never checking truth, only presence.
- **It requires every future contributor to remember the ritual, forever, with no enforcement
  beyond "does the string exist."** That's the same failure shape as "keep the docs updated by
  hand" — a process a linter can't actually save you from, because the writing of the pointer is
  itself an unenforced human step.
- **It doesn't work at all for a real class of decisions.** Not every decision is realized in
  application code that can hold a comment: infrastructure changed by hand in a cloud console,
  contract code whose own convention forbids this exact comment style and requires a different
  versioning scheme instead (a public contracts mirror is one concrete case), a CI/CD YAML workflow,
  a decision about *process* (adopt a documentation convention) whose realization is a README and a
  directory structure, not a function. All of these are permanent, structural blind spots under a
  code-comment-only model — not edge cases to special-case around, but proof the location assumption
  is wrong.

**Revised model: the record-side pointer is the universal addressing mechanism; everything else is
a ranked confidence ladder on top of it, not a set of competing approaches to pick once and commit
to.** Every ADR/Spec carries `realized_by: [{type, locator, verification?, owner?, cadence?, ...}]`
in its own header/frontmatter. `type` distinguishes what's being pointed at (`code`, `infra_manual`,
`process`, `external`, open to whatever a profile needs); `locator` is a file/glob for code, a
description for a manual/process claim. The record always carries the claim. Confidence isn't
binary, and comment-in-code is explicitly **not** the strongest available signal (informed by prior
art on graduated provenance — SLSA/in-toto's levels, ArchUnit-style fitness functions, and the
SOC 2/CODEOWNERS attestation model). Ranked weakest to strongest, for the `code` type:

1. **Pointer only, unchanged since last verification (via git history on the locator).** Cheapest
   check, no code touched, no LLM call — "nothing has happened here that could have invalidated
   this" is a real, useful, low-cost signal on its own, exactly the "hasn't changed in a year,
   probably still fine" heuristic, and the tier that makes record-only backfill viable at all:
   reconstructing history only ever writes to the decision-record file, never touches application
   source.
2. **Pointer + inline comment, both present and unchanged.** Same cost to check (still a grep,
   still no LLM), marginally higher confidence than tier 1 — the comment is evidence a human
   deliberately anchored the claim to this exact spot, not just "somewhere in this file." Purely
   additive and purely optional: a profile or team that likes the deterministic feel of
   comment-grep keeps getting it at no extra cost, without it ever being required — but it does not
   outrank tier 3, and shouldn't be reached for as the default just because it's familiar. It's
   genuinely a weak signal: it proves a string was typed once, not that the claim still holds (see
   the diagnosis above).
3. **Pointer + passing executable verification** — an ArchUnit-style fitness function, a
   dependency-cruiser rule, a contract test, anything that mechanically *tests* the claimed
   property rather than merely pointing near it. Strongest confidence tier a `realized_by` entry
   can carry, because it's the only one that verifies the property is actually true rather than
   checking for evidence someone once claimed it was. Real cost — someone has to write and
   maintain the check — but it produces the same artifact a good test suite already wants, not
   pure governance overhead.
4. **Locator changed since last verification, at any of the tiers above.** The shared escalation
   path — staleness, never comment-presence-or-absence, is what triggers a periodic,
   LLM-assisted re-verification pass (the drift-detection alarm's investigation step already
   scopes inference to; see RFC-0002/RFC-0003 for the concrete detect/apply and JSON-output
   mechanics this triggers in practice). The agent visits the locator and makes the actual
   judgment call the mechanical tiers were never claiming to make on their own.
5. **`infra_manual`/`process` locators — never mechanically checked, held to the same discipline
   as a SOC 2/ISO 27001 control review, not just a one-time note.** Each carries a named
   accountable `owner` and a review `cadence`, scoped by glob the same way CODEOWNERS scopes
   responsibility — not one global attestation policy for an entire repo. Goes stale on the
   cadence expiring, independent of whether anything is known to have changed, since there's no
   artifact to diff.

**Why the inline comment can't be the required baseline, even as one supported option among
others:** backfill. Reconstructing decision history for an existing codebase under a
comment-required model means programmatically editing however many application source files
across however many languages — itself a large, invasive, error-prone operation prone to silent
corruption at exactly this scale. A record-only pointer makes backfill strictly safer: it only ever writes to the artifact
Urzua owns. Comments stay available and encouraged where a team wants them — they just can't be
*required*, or backfill inherits their invasiveness.

Whether a given profile or team expects/requires the inline-comment tier is itself a per-profile
(or even per-`type`) setting, not a global tool decision — accommodates languages/conventions that
forbid or discourage this exact comment style without forcing them onto a lower-confidence tier
than teams whose conventions are comment-friendly.

Non-goal here: this RFC does not attempt to fully design the profile schema for `realized_by`
entries, the staleness-cadence policy, or the re-verification trigger mechanism in general — RFC-
0002 designs the concrete detect/apply mechanism for one specific instance of tier 4's
re-verification trigger; the general staleness-cadence policy across all tiers/locator types
remains open.

### 3. `realized_by` is a claim graph, not a flat list

A flat list of `realized_by` entries doesn't generalize. A decision that's genuinely realized in
several required parts — consider an ADR that broke exactly this way: DI plumbing present, the
actual exchange call it depended on absent — needs the computed confidence to be capped by its
weakest required part, not hidden behind its strongest one. A flat list with an implicit "take the
max" can't express that a claim has required sub-parts at all.

**Model: `realized_by` is a small graph of claims, not a list.** A **leaf** claim is exactly what
§2 above already defined — `{type, locator, tier-evidence, owner/cadence if attestation-based}`. A
**composite** claim is `{operator: AND | OR, children: [claim, ...]}`, where each child is itself
a leaf or another composite — recursive, to a depth bounded by this section's own config (below).
Evaluation is one recursive rule: an `AND` node's confidence is the **minimum** of its children's;
an `OR` node's is the **maximum**. A bare leaf is the degenerate one-node case — the overwhelmingly
common shape, and it costs an author nothing beyond what §2 already asked for.

**It's a graph, not a tree, because claims need to be shareable across records, not just nested
within one.** Consider three ADRs (`ADR-0072`/`0073`/`0074`) independently citing — and
independently going stale on — references to substantially the same underlying migration work.
A tree can't fix that; each composite owns its children uniquely, so three ADRs get three copies
of essentially the same evidence, free to drift apart exactly as they did. Under a graph, a claim
can be **promoted** to a standalone, addressable node (its own `id`) that multiple records' claim
graphs reference by pointer instead of duplicating — one staleness event instead of three silent,
unrelated ones. Promotion is a normal revision-log event on whichever record first needed to share
it, not something an author has to plan for up front — most claims stay anonymous inline leaves
forever.

**This pushes the same graph-shaped thinking the core schema already uses at the record level
(`related`/`supersedes`/`derives_from`/`implements`, per RFC-0001) down to the claim level** —
consistent, not a second paradigm bolted on: records reference records, claims reference claims,
and a record's `realized_by` field is itself an entry point into the claim graph.

**Consequences for the engine, stated honestly rather than deferred silently:**

- Evaluation needs memoization (a shared node reached via two paths computes once) and **cycle
  detection** (a claim cannot transitively depend on itself) — solved graph-algorithm territory,
  but real engine work, not free.
- A new query surface is now genuinely useful, not just nice-to-have: "what depends on this
  claim" — exactly what would have caught the ADR-0072/0073/0074 drift proactively rather than
  three times independently after the fact.
- **Staged, not built all at once:** the schema is graph-native from the start (claims are
  addressable, cross-record references are legal), but a first linter implementation only needs
  to handle the no-sharing, single-record tree case — nothing forces early adoption of
  promoted/shared nodes, and that machinery is worth building once there's real demand for it
  (there already is — see above), not before.

**Depth is a configurable parameter, not a fixed limit — shipped with a sane default, not
unbounded.** A profile (or an org, at the tool-config level) can raise or lower the max nesting
depth `realized_by` accepts; Urzua ships a default rather than requiring every adopter to decide
this on day one. Proposed default: **4** — generous enough for a genuinely multi-part decision,
shallow enough that hitting the limit reads as a signal ("this ADR is scoped too broadly, consider
splitting it") rather than a wall the model needs to be deeper to avoid. Not load-bearing to this
RFC which exact number ships; the load-bearing part is that it's configurable at all, consistent
with RFC-0001 §1a's provider model — the tool shouldn't hardcode an org's tolerance for decision
complexity any more than it should hardcode the set of record types.

**Left explicitly open, not resolved here:**

- A `NOT`/negative-condition operator — neither `AND` nor `OR` cleanly expresses "this claim holds
  as long as X never happened," which is exactly the shape of an escape-hatch decision with no
  mechanism tracking whether it was ever invoked in production. Whether claim-graph `NOT` is the
  right vehicle for that, or it needs its own event-tracking mechanism entirely separate from
  realization claims, is undecided.
- The exact promotion mechanic — what triggers a leaf being extracted into a shared, addressable
  node (author-initiated, or does the engine ever suggest it after detecting near-duplicate claims
  across records?) — is not designed here, only the schema shape that makes promotion possible
  later.

## Open questions

- **MVP scope, decided rather than left implicit.** §3's full claim graph (AND/OR composites,
  weakest-link confidence, cycle detection) ships once a real multi-part-AND case is found needing
  it — not before. The MVP that does ship: `realized_by` as categorized locator lists (`spec`/`code`/
  `test`), Embodiment computed as the highest tier with a non-empty category (any `test` locator →
  `Verified`; else any `code` locator → `Implemented`; else any `spec` locator → `Specified`; else
  `Not started`); a leaf can be **promoted** into its own first-class `claim` record (ADR-0018) when
  the same locator is cited by more than one record, so one staleness event covers every citing
  record instead of each independently drifting; promotion is **engine-suggested** — `check` flags
  the duplication as a finding, a human runs the actual promotion, never automatic. No AND semantics,
  no cycle detection, no `NOT` operator in this MVP.
- Should `Embodiment`'s back-pointer computation stay purely mechanical (grep-based), or is there
  a role for LLM-assisted "does the code really match the ADR's intent" checking, and if so, at
  what point in the pipeline (base computation vs. a separate confidence-scored second pass)?
  Leaning mechanical-only for the base computation, but not fully closing this. Partially
  resolved: mechanical-only for base computation stands, but base computation is now
  presence/staleness of a record-side pointer, not a code-side comment — and staleness (not
  presence) is what triggers the still-open LLM-assisted truth-check (RFC-0002 designs the concrete
  detect/apply mechanics for this trigger, not the truth-check itself).
- What does "Spec drifted enough to need a new ADR" actually look like operationally — is this a
  judgment call surfaced to a human, or can Embodiment's existing `Drift detected` alarm state
  double as the trigger? See RFC-0006 (living-spec problem) for the other half of this question.

## Non-goals

Does not attempt to design the paging/notification mechanism, dashboard/UI, or storage technology
— same scope boundary as RFC-0001. Does not fully design the profile schema for `realized_by`
entries in general, the staleness-cadence policy across all tiers, or a general re-verification
trigger mechanism beyond what RFC-0002 already designed for one concrete case.

## References

- RFC-0001 — the core schema/profile model `realized_by` and Embodiment sit on top of.
- RFC-0002 — the concrete detect/apply refresh mechanism for tier 4's re-verification trigger.
- RFC-0003 — agent-native JSON output, referenced for how this content's detect/apply tooling
  surfaces its results.
- Terraform's provider architecture and Kubernetes CRDs — prior art for the pointer-on-record, not
  embedded-in-code, model (see RFC-0001 §1a).
- SLSA/in-toto's graduated provenance levels, ArchUnit-style fitness functions, and the
  SOC 2/CODEOWNERS attestation model — prior art for the confidence ladder in §2.
