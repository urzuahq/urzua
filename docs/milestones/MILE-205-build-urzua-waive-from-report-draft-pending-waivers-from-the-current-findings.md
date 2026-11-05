---
Stable-Id: 01M6PZ7D13HCWEF52VDBARX38Q
Status: Planned
Phase: '1'
Track: schema-governance
Implements: ADR-91
Blocked-on: null
---
# 205 — Build urzua waive --from-report: draft pending waivers from the current findings

## What

Build `urzua waive --from-report` of `ADR-91`: it drafts one waiver per rule and file for the current
unwaived findings, in the repository's waiver format, with a pending `Reason` and no `Expires` or
`Category`, dry-run unless `--apply`, filtered by `--rule` and `--path`, reporting as JSON what it wrote.

## Why

Phase 1 is where a real adopter's corpus exists and the tool is judged on getting that corpus to zero
findings (`MILE-102`, `MILE-118`). Adopting urzua on a second real corpus took 91 hand-written waivers. The
maintainer decided on 2026-11-05 that waivers stay the only suppression mechanism and there is no baseline
file, so adoption needs this command or it needs 91 transcriptions. It serves adoption, not the Phase 0
engine, so it belongs in Phase 1.

## Done means

- A suppression type may declare `statuses: {pending: [...]}` without `values`; a type declaring `values`
  loads as before. Both the serde default (`status_model.rs:35`) and the empty-list check (`:368`) are relaxed,
  and a test fails before and passes after. Every reader of `values` is checked and covered: `decision_types`
  (`config.rs:1597`), `StatusDecl::declares`, `status_model.rs:263` and `:437`, `commands/new.rs:180`,
  `config.rs:2937` and `rules/status.rs:308`; any that breaks on an empty list is fixed.
- `urzua waive --from-report` without `--apply` writes nothing, and prints JSON with `status`, `mode`,
  `pending_value`, `pending_source`, `drafts`, `skipped` and `unmatched_filters`; with `--apply` it writes the
  drafts and each `drafts[].written` is true. Tests plant findings in two files and three rules and assert one
  draft per rule and file, numbered after the highest existing waiver and fitted to the declared identity.
- A draft has `Reason` set to the type's first declared whole-value pending entry, else `Pending`, with
  `pending_source` saying which; `Expires`, `Category` and `Justified-by` are `null`. `urzua check` then reports
  the original finding again and `waiver-ineffective` on the draft, and exits 1 under `fail_on: warning`;
  after a real `Reason` and `Expires` are written the finding is waived.
- On a type declaring `permanent`, a draft whose author fills only `Reason` is still ineffective
  (`RuleNotListed` or `CategoryMissing`), asserted by a test.
- The command exits 2 and writes nothing when `Reason` is not in `effective_requires`, when the chosen value
  classifies as `Present`, when `pending` holds only patterns, or when the type neither declares `permanent`
  nor lists `Expires` in `effective_requires`; the last message names the fix (declare `permanent`, or require
  `Expires`). Each is a test.
- Findings of `waiver.matches-nothing`, findings with no file, and findings already named by a waiver with the
  same rule and scope are listed in `skipped` with a reason; a second `--apply` writes nothing.
- Info findings are skipped and not listed in `skipped`.
- `--rule` with an unknown id exits 2; `--path` takes plain paths (a file or directory prefix, no glob); a filter that matches nothing is
  in `unmatched_filters`.
- The scaffold is shared with `urzua new`, not duplicated; `new`'s tests are unchanged.
- The command is exercised in this repository: `urzua waive --from-report` on a scratch copy with a planted
  finding drafts a waiver, and `urzua check` and `check --base origin/main` show 0 unwaived findings on the
  merged tree.
- `SPEC-10`, `SPEC-3`, `SPEC-12` and a new spec describe the command; `make records` and `make rule-table-check` pass.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** Maintainer decision, 2026-11-05, recorded in `ADR-91` (Proposed). | **substantive** |
> | 2026-11-05 | Renumbered from `MILE-204` to `MILE-205`: two parallel PRs (#362, #364) both took 204, and the harness cutover milestone keeps it. **Why:** an id is unique per type. | **structural** |
> | 2026-11-05 | Amended with `ADR-91` after review: both `status_model.rs:35` and `:368` are relaxed and the readers of `values` checked; the command also refuses without `permanent` or `Expires` required; info findings skipped; `--path` is plain paths. Phase 1 confirmed. **Why:** independent review of PR #364. | **substantive** |
