---
Stable-Id: 01M7A3B7TD1N87WAEFWDMB5CTJ
Status: Draft
Date: 2026-11-13
Version: '0.1'
Author: beauwilliams
Subject: '`urzua waive --from-report` -- drafting one waiver per rule and file from the current findings.'
Implements: ADR-91
Parent: SPEC-1
---
# SPEC-25 — urzua waive --from-report

## Purpose

`urzua waive --from-report` removes the transcription from adopting urzua on a corpus that already has
findings: it drafts one waiver per rule and file for the current unwaived findings, in the repository's
own waiver format, with a pending `Reason` and no `Expires`, `Category` or `Justified-by`. A draft
suppresses nothing until a person writes the reason and the date, so the command adds no baseline
file and hides no finding (`ADR-91`, `ADR-11`, `ADR-86`). This spec is the reference for the command as
built; `MILE-205` tracks what of it is built.

## Command

```
urzua waive --from-report [--rule ID[,ID]]... [--path PATH]... [--by NAME] [--config FILE]
```

- `--from-report` is required. It selects the source, the report of a `check` run in process over the
  whole corpus; no saved report is read, so a draft is never built from a stale one. A saved-report
  input is not offered (maintainer decision of 2026-11-13).
- This build previews only: it writes nothing and its `mode` is always `dry-run`. `--apply`, which
  writes the drafts, is the next change to `MILE-205` and is not accepted yet.
- `--rule` narrows to the named rules; an id this build does not ship is a could-not-run (exit 2), as for
  `check --rules`. `--path` is a plain path, a file or a directory prefix compared by whole path
  component, never a glob, relative to the repository root and repeatable.
- `--by` names who ran it, as for `new`; it is recorded in each draft's revision-log row. A draft also
  carries an `Owner` header, set to that name, only when the waiver template lists `Owner` and the
  type declares it.

Exit `0` whenever the command ran, with or without drafts; `2` when it could not run. It never exits `1`:
whether the repository is clean is `check`'s answer. Stdout is one JSON document; notices go to stderr.

## Which findings are drafted

A finding is a candidate when it is not waived and is at or above the effective `fail_on`. Findings
below it, including every `info` finding, are neither drafted nor listed. Candidates are grouped by
rule and file: one draft per group, `Scope` the finding's path exactly as a waiver compares it, never
`*`. A candidate that is not drafted is listed in `skipped` with a `reason`:

| `reason` | The finding |
|---|---|
| `matches-nothing` | is of `waiver.matches-nothing`: a waiver for a waiver would hide the report that says a waiver does nothing |
| `no-file` | is tied to no file, so no scope names it |
| `already-waived` | has a waiver record with the same `Rule` and `Scope` that is not expired (it takes effect, or is still ineffective); `waiver` names it |
| `expired` | has such a waiver whose `Expires` has passed or is not a date; `waiver` names it, and it is renewed, not drafted again |
| `unwaivable` | is of a variant its rule declares unwaivable, which no waiver can cover |
| `resolved-by-drafts` | disappears when the drafts exist, judged on the tree with the drafts held in memory; the first waiver ending a "waiver directory matches nothing" finding is the case (`ADR-91`, amendment of 2026-11-08) |

A second run therefore drafts nothing for a finding a draft already names. A `--rule` or `--path`
that matches no candidate is listed in `unmatched_filters`, never silently ignored.

## The draft

The record is produced by the scaffold `urzua new` uses (`SPEC-12`, `scaffold_record`): the next
number past every directory the type declares, fitted to its declared identity, a minted `Stable-Id`,
the type's template body and a first revision-log row. On top of it the command supplies `Rule`, `Scope`
and `Reason` (the type's first whole-value `statuses.pending` entry, else `Pending`) and appends a
`## Findings` section listing the finding messages verbatim, with no cap. The title is `<rule> in <file>`.
A dry run shows the number each draft would take now, numbered consecutively in file then rule order.

## Refusals (exit 2, nothing written)

Before `check` runs the command proves a draft would suppress nothing, and refuses when:

- `Reason` is not in the suppression type's `effective_requires`, since a pending `Reason` would then
  suppress its finding;
- the chosen value reads as a real `Reason` (a present value) for the type;
- the type's `statuses.pending` holds only patterns, so there is no whole value to write;
- the type neither declares `permanent` nor lists `Expires` in `effective_requires`: a draft given
  only a `Reason` would be a permanent waiver by accident. The message names both fixes.
- the type declares a closed field set that omits `Rule`, `Scope` or `Reason`.

## Output

```json
{"status":"ok","mode":"dry-run","pending_value":"Pending","pending_source":"declared|default",
 "drafts":[{"path":"docs/waiver/WAIVER-0053-...md","stable_id":null,"rule":"x.y","scope":"docs/a.md",
            "findings":{"count":2,"lines":["docs/a.md:12 message"]},"written":false}],
 "skipped":[{"rule":"..","file":"..","reason":"already-waived","waiver":"docs/waiver/WAIVER-0007-..md"}],
 "unmatched_filters":[{"kind":"rule|path","value":".."}]}
```

`stable_id` is `null` in a dry run: the id is minted when the record is written. `file` of a skipped
entry is `null` for `no-file`; `waiver` is present only for `already-waived` and `expired`.

## What's deliberately not built

- **`--apply`.** Writing the drafts, retaking a number another file claimed, idempotence, and the
  check-after-apply behaviour belong to a later change of `MILE-205`.
- **A prefilled `Expires`**, a `--expires` flag or a default expiry in configuration: a dated draft with
  a real `Reason` becomes effective, and the date is the human's choice (`ADR-86`).
- **A saved-report input** (a file or stdin): a saved report can be stale.
- **A `--min-severity` flag**, a non-zero exit when drafts exist, and a `--dry-run` alias: `check` owns the
  gate, and preview is the default as for `index` and `fix`.
- **A cap on the findings listed in a draft's body.**

## References

- `ADR-91` -- the decision; `ADR-11`, `ADR-86`, `ADR-64`, `ADR-70`.
- `SPEC-10` (the waiver record type), `SPEC-12` (the shared scaffold), `SPEC-3` (`statuses.pending`,
  `effective_requires`, `permanent`), `SPEC-1`.
- `MILE-205` -- builds this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-13 | Initial spec: the preview of `urzua waive --from-report` as built (eligibility, grouping, the pending value, refusals, the JSON report); `--apply` is not built. **Why:** `MILE-205`, plan PR3, maintainer decisions of 2026-11-13. | **structural** |
