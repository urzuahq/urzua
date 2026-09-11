---
Stable-Id: 01M28H5J6J3YN1WNGS7HHNR6JY
Status: Draft
Date: 2026-09-11
Author: beauwilliams
---
# 26 — Records, not GitHub issues: close the issue tracker as a tracking surface

## Summary

This project tracks its own work as `urzua` records — milestones, bugs, RFCs, ADRs, specs — in
`docs/`, checked by the tool it is building. GitHub issues were used once, at the very beginning,
and never since. Propose making that explicit: GitHub issues are not a tracking surface for this
repository, and the issue templates are repointed or removed accordingly.

This is not purely a codification of existing practice, and the first draft of this RFC wrongly
claimed it was. `CONTRIBUTING.md` actively invites issues in two places — line 3 ("issues and PRs
are triaged") and line 29, which lists "Reading the RFCs and filing an issue with a specific
objection" as the *first* item under "What's useful right now". Adopting this RFC means **reversing
a checked-in, human-facing invitation**, not just writing down something everyone already did.

## Motivation

Evidence, not preference:

- **One issue has ever been opened** (#1, 2026-09-05, "Phase 8.1: every tested foreign ADR corpus
  fails 100% on `header.required-fields`"). Nothing has been filed since — 6 days and 30 pull
  requests later. (It has since been closed and its content migrated; see Proposal point 2.)
- Meanwhile the corpus grew to **20 bug records, 26 RFCs, 45 ADRs, and 93 milestones** — counts as
  of this RFC's own merge, including the three records filed alongside it. Every piece of real
  tracking this project does already happens in `docs/`.
- **AGENTS.md already mandates the record path** and says nothing about issues: *"This project's own
  backlog is tracked through `urzua`'s own record types, not scratch notes or conversation
  history."* The instruction is unambiguous for agents and silent for humans, which is how issue #1
  came to exist at all.
- The split is not free. Issue #1's content is genuinely useful — a falsifiable test against two
  foreign ADR corpora — and it sat outside the corpus for six days, invisible to `urzua check`,
  `urzua graph`, and `urzua explain`.
- **A caveat this RFC has to own**: migration alone does not deliver that benefit. BUG-23, the record
  issue #1's content became, carries no `Implements` pointer, so `urzua graph` gains zero edges from
  it and `urzua explain` returns nothing — it is as graph-isolated as the issue was. Being inside
  `docs/` makes a finding *checkable* (it must satisfy its type's schema, and `check` will fail the
  build if it doesn't); it does not make it *connected*. Claiming the latter would overstate what
  moving a record across a boundary actually buys.
- `.github/ISSUE_TEMPLATE/bug_report.md` covers the same ground as the `bug` record type in a second,
  unchecked format. Not the same schema — its fields (What happened / What you expected /
  Reproduction / Version) share nothing with `bug`'s (`Status`/`Found-in`/`Regression-test`/
  `Realized-by`), and they serve different audiences. The objection is narrower than "duplication":
  one of the two is validated by a rule and one is not.

## Proposal

1. **State the policy.** Issues are not used for tracking. Work is filed as the record type that
   fits: `bug` for a defect, `milestone` for planned work, `rfc` for a proposal, `adr` for a
   decision. Add this to `AGENTS.md` and `CONTRIBUTING.md` so it binds humans as explicitly as
   AGENTS.md already binds agents.
2. **Migrate and close issue #1** — *already done*, at the maintainer's explicit direction, before
   this RFC was written. Recorded here rather than proposed, so the RFC is not read as seeking
   permission for something already irreversible. Its two halves are separable and neither is lost:
   - the heading-delimited / front-matter-only header-shape question is already tracked by
     **MILE-22** (MADR/Nygard import, `Status: Planned`);
   - the `title_number()` message defect is now **BUG-23**, filed against live code re-verified on
     2026-09-11.
   Close the issue with a comment pointing at both, so the trail from the old surface to the new
   one is readable.
3. **Decide the templates' fate, and CONTRIBUTING.md's with them.** `.github/ISSUE_TEMPLATE/` holds
   two files, not one: `bug_report.md` and `design_feedback.md`. The latter exists specifically to
   serve `CONTRIBUTING.md:29`'s invitation to file objections against RFCs, so deleting it is a
   bigger change than tidying a duplicate — it removes the only structured channel for design
   feedback from someone without commit access. Whatever is decided, `CONTRIBUTING.md:3` and `:29`
   must change in the same commit, or the repository will state two contradictory policies.
4. **Keep issues enabled, read-only in practice.** External adopters do not exist yet (AGENTS.md's
   "no external adopters" premise), but if one arrives, an issue is the only channel they have
   before they can open a PR. Disabling issues entirely would close that door; the policy is about
   what *this project* tracks, not about refusing inbound contact.

## Open questions

- **Does this survive first contact with an external adopter?** The whole premise holds while the
  contributor set is one person plus agents. A drive-by reporter cannot be asked to learn a record
  schema before reporting a crash. Point 4 hedges, but the honest answer is that this policy may
  need revisiting the first time someone outside files something — and that is a reason to write it
  down now, not to avoid deciding.
- **Is `bug` the right type for an inbound external report at all?** A `bug` record requires
  `Found-in` and `Regression-test`; a stranger's report has neither. Possibly inbound reports land
  as issues and a maintainer converts them, which is a different policy than "issues are unused."
- **Should this be an ADR instead?** It is a decision about process, not about the tool's design.
  ADR-41's "coherent feature area, decided editorially" test may or may not cover process decisions;
  this is filed as an RFC because the question of whether it is even ADR-shaped is itself open.

## Non-goals

- Not proposing any change to how records themselves are structured, numbered, or checked.
- Not proposing GitHub Projects, Discussions, or any other tracker as a replacement — the record
  corpus is the tracker.
- Not proposing to disable issues as a GitHub feature (see point 4).
- Not addressing pull requests, which stay exactly as they are: AGENTS.md's git workflow already
  requires a PR for every change, and nothing here touches that.

## References

- GitHub issue #1 — the one issue ever filed, the evidence this surface is unused, and the content
  this RFC migrates.
- MILE-22 — MADR/Nygard import; issue #1's header-shape half.
- BUG-23 — issue #1's `title_number()` message half, filed against re-verified live code.
- AGENTS.md, "Use the tool on itself" — the existing agent-facing mandate this RFC would extend to
  humans.
- SPEC-9 — the `bug` record type whose schema `.github/ISSUE_TEMPLATE/bug_report.md` duplicates.
- ADR-41 — the editorial-coherence test relevant to the open question of whether this should be an
  ADR.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-11 | Initial RFC, `Status: Draft`. Filed while closing GitHub issue #1; the policy was already real in practice and in AGENTS.md, but never written down as a decision anyone could point at. | **structural** |
> | 2026-09-11 | Corrected before merge, from an adversarial review. (1) The RFC claimed the policy was merely unwritten; `CONTRIBUTING.md:3` and `:29` actively invite issues, so adopting this reverses a stated invitation rather than codifying practice -- now said plainly in the Summary and folded into Proposal point 3. (2) Three of four corpus counts were wrong or stale ("46 ADRs" when there are 45; bug/RFC counts predating this RFC's own commit), in a section headed "Evidence, not preference". (3) The claim that `bug_report.md` duplicates SPEC-9's schema was overstated -- zero fields overlap; narrowed to the real objection, that one of the two is rule-validated and one is not. (4) Proposal point 2 proposed closing issue #1, which the maintainer had already directed and which was already done; reframed as recorded rather than proposed. (5) Added the caveat that migration buys checkability, not connectedness -- BUG-23 carries no `Implements` pointer and adds zero `urzua graph` edges, so the RFC's own stated benefit is not delivered by its own migration. (6) `design_feedback.md` was unnamed despite point 3 proposing to delete the directory holding it. | **substantive** |
