---
Stable-Id: 01M28H5J6J3YN1WNGS7HHNR6JY
Status: Draft
Date: 2026-09-11
Author: beauwilliams
Implements: —
---
# 26 — Records, not GitHub issues: close the issue tracker as a tracking surface

## Summary

This project tracks its own work as `urzua` records — milestones, bugs, RFCs, ADRs, specs — in
`docs/`, checked by the tool it is building. GitHub issues were used once, at the very beginning,
and never since. Propose making that explicit: GitHub issues are not a tracking surface for this
repository, the one open issue is migrated into records and closed, and the issue templates are
either removed or repointed at the record types they duplicate.

## Motivation

Evidence, not preference:

- **One issue has ever been opened** (#1, 2026-09-05, "Phase 8.1: every tested foreign ADR corpus
  fails 100% on `header.required-fields`"). It has one comment, from the same author, eight hours
  later. Nothing has been filed since — 6 days and 30 pull requests later.
- Meanwhile the corpus grew to **18 bug records, 25 RFCs, 46 ADRs, and 90+ milestones**, all filed
  as records. Every piece of real tracking this project does already happens in `docs/`.
- **AGENTS.md already mandates the record path** and says nothing about issues: *"This project's own
  backlog is tracked through `urzua`'s own record types, not scratch notes or conversation
  history."* The instruction is unambiguous for agents and silent for humans, which is how issue #1
  came to exist at all.
- The split is not free. Issue #1's content is genuinely useful — a falsifiable test against three
  foreign corpora — and it has sat outside the corpus for six days, invisible to `urzua check`,
  `urzua graph`, and `urzua explain`. A finding that cannot be pointed at by a `Implements` edge is
  a finding the tool cannot help anyone act on. That is the concrete cost, and it is this project's
  own dogfooding thesis applied to its own backlog.
- `.github/ISSUE_TEMPLATE/bug_report.md` duplicates the `bug` record type's own schema
  (`.urzua/templates/bug.md`, SPEC-9) in a second, unchecked format. Two shapes for one concept, one
  of which no rule validates — the same defect this project exists to remove from ADR corpora.

## Proposal

1. **State the policy.** Issues are not used for tracking. Work is filed as the record type that
   fits: `bug` for a defect, `milestone` for planned work, `rfc` for a proposal, `adr` for a
   decision. Add this to `AGENTS.md` and `CONTRIBUTING.md` so it binds humans as explicitly as
   AGENTS.md already binds agents.
2. **Migrate and close issue #1.** Its two halves are separable and neither is lost:
   - the heading-delimited / front-matter-only header-shape question is already tracked by
     **MILE-22** (MADR/Nygard import, `Status: Planned`);
   - the `title_number()` message defect is now **BUG-23**, filed against live code re-verified on
     2026-09-11.
   Close the issue with a comment pointing at both, so the trail from the old surface to the new
   one is readable.
3. **Decide the templates' fate.** Either delete `.github/ISSUE_TEMPLATE/` outright, or replace it
   with a single stub directing a reporter to open a PR adding a `bug` record. Leaving a working
   bug-report template while declaring issues unused is the worse of the three options — it invites
   exactly the thing the policy forbids.
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
