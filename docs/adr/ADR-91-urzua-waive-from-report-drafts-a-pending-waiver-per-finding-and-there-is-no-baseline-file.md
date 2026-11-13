---
Stable-Id: 01M6PZ7CBKS3SY50FX92SA221S
Status: Accepted
Accepted-on: 2026-11-06
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: ADR-11, ADR-86, ADR-64, ADR-70
---
# 91 — urzua waive --from-report drafts a pending waiver per finding, and there is no baseline file

## Context

Line references are to `origin/main` at `a824c85a`. `commands/` and `main.rs` are under
`rust/crates/urzua-cli/src`; `waiver.rs`, `field_state.rs`, `status_model.rs` and `rules/` are under
`rust/crates/urzua-core/src`.

Adopting urzua on a second real corpus needed 91 time-boxed waivers, written by hand, to reach zero
warnings (the maintainer's report, 2026-11-05). Each one is the same work: read a finding, make a file
under the waiver directory with the next number, copy `Rule` and `Scope`, then think of a reason and a
date. Only the last step needs a person. The rest is transcription, and 91 repetitions of it is a cost
that falls on the first hour of adoption, which is when a tool is judged.

The maintainer decided on 2026-11-05 that **waivers stay the only suppression mechanism and there is no
baseline file** ("waivers are the tool for this; a baseline sounds inconsistent"). This record settles
how the transcription is removed without weakening that.

What exists today:

- **A waiver is a record, numbered and scaffolded by `urzua new`.** `new` scans every directory the type
  declares for the highest number and takes the next, fitted to the type's declared identity pattern
  (`commands/new.rs:297`, filename at `:120-150`), mints a `Stable-Id` (`:155`), writes the type's header
  fields as `null` and the template body, and refuses to overwrite (`create_new`, `:241`). For `waiver` it
  writes `Rule`, `Scope`, `Reason`, `Expires`, `Category` and `Justified-by` all `null` (observed).
- **A waiver covers a finding by exact rule and exact file.** `Waiver::covers` matches `Rule` exactly and
  `Scope` as `*` or the finding's path exactly (`waiver.rs:180`); `apply_suppressions` marks only
  active, effective waivers (`:316-327`). So one waiver per rule and file is the natural grain.
- **A waiver can be loaded and still do nothing.** `load_suppressions` reads every waiver record and records
  which `effective_requires` fields it lacks (`waiver.rs:215-222`). A field is unmet unless
  `classify_with` says `Present`, so absent, blank, placeholder and pending all fail it (`:219`).
  `is_effective` is false while any is unmet or while `permanent` refuses it (`:168-170`), and such a waiver
  is skipped by `apply_suppressions`, so its finding stays. `waiver.matches-nothing` reports it as
  `waiver-ineffective`, naming the unmet field (`rules/waiver.rs:76-107`). This repository enables that rule
  at `warn` (`.urzua/config.yaml:519`) and declares `effective_requires: [Reason]` (`:349`), under
  `fail_on: warning` (`:4`).
- **There is no pending vocabulary for a waiver today, and none can be declared.** The `statuses.pending`
  list is the declared source of what "pending" means for a type (`status_model.rs:70`, read by
  `is_pending_value`, `:208`). A declaration without `values` fails twice: first in serde, because
  `StatusDecl.values` has no default (`status_model.rs:35`, the "missing field `values`" observed here), and
  then, were that relaxed, in the empty-list check (`check_list("values", ...)`, `:368`). A waiver has no
  `Status`, so its type declares no `statuses` (`config.yaml:340-382`). With none declared, `classify_with` falls back to the compiled-in match: any
  value containing `pending`, or `todo` (`field_state.rs:56-58`), plus the placeholder tokens such as `tbd`
  (`:17-26`). Observed on this repository: a waiver whose `Reason` is `Pending`, `TBD` or `null` is
  reported `waiver-ineffective` for lacking `Reason`.
- **Permanence compounds it.** A waiver without `Expires` is governed by the declared `permanent` list
  (`ADR-86`, `waiver.rs:243`). Here the same three waivers also read "it has no `Expires` and its type
  declares `permanent` without an entry for 'field.quality'" (observed). So a draft that omits `Expires`
  and `Category` can never take effect by accident, for any rule this repository does not list.
- **`SPEC-14` already names this tool and leaves it open.** `migrate schema --assist-waivers` is "blocked
  on deciding whether a generated draft preserves the review property" of the waiver model
  (`docs/specs/SPEC-14-urzua-migrate.md:100-103`). Part of what follows answers that question.
- **Write commands are dry-run unless told otherwise.** `index` and `fix` write only with `--apply`
  (`main.rs:206`, `:237`). A new write command follows the same shape.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Hand-write waivers (today) | Nothing to build; every waiver is authored | 91 transcriptions in the first hour of adoption; numbering and header typos; the cost scales with the corpus, so it is highest for the adopters most worth having |
| B. A baseline file (a committed list of today's findings that `check` ignores) | Familiar from other linters; one file; zero effort | Rejected by the maintainer: waivers are the tool for this and a baseline is inconsistent with them. A second suppression mechanism needs its own expiry, scope, review and reporting, which is what a waiver already is. A baseline is an ignore list, which `ADR-11` refused because it is unreviewable and grows forever |
| C. Demote the noisy rules to `info` | One line of config per rule; no records | Silences every future finding of the rule, not today's; the demotion is a repository-wide statement made to clear a one-time backlog; nothing records why, until when, or for which files. A rule at `info` cannot gate again without another edit |
| D. A generator that writes *effective* waivers with a stock reason | Zero effort; reaches zero warnings immediately | Invents the one thing a waiver exists to carry. `Reason` is "the whole reviewability point of not using an ignore list" (`SPEC-10`); a stock reason makes it a baseline in waiver clothing |
| E. A generator that writes *ineffective* drafts and leaves the reason, expiry and category to a person (chosen) | Removes the transcription; no finding is hidden until a human acts; reuses `new`'s numbering, template and identity; same record, same review as a hand-written waiver | A new command; a repository is not at zero warnings until each draft is filled in (by design) |

## Decision

*In the context of an adopter whose first run produced 91 findings that each needed a waiver, facing a
maintainer decision that a waiver is the only suppression mechanism and a baseline file is out, we decided
that `urzua waive --from-report` drafts one waiver per rule and file for the current findings, in the
repository's own waiver format, with a pending `Reason` and no `Expires` or `Category`, to achieve the
adoption path without a second mechanism or an unreviewed suppression, accepting that a draft suppresses
nothing until a person fills it in.*

1. **No baseline file, no second suppression mechanism.** Suppression stays the waiver record
   (`ADR-11`, `SPEC-10`). Nothing in this decision adds a file `check` reads to ignore findings.
2. **The command.** `urzua waive --from-report` is a new top-level command. It runs the same checks `urzua check`
   runs, in process, over the whole corpus, and drafts from that report; it reads no saved report, so a
   draft is never built from a stale one. The name is settled here because the flag selects the source
   (the current report) and leaves room for others without renaming the command. It takes no other source today.
3. **Which findings.** A finding is drafted when it is unwaived and would fail the gate, that is at or
   above the effective `fail_on` (`commands/check.rs:399`). It is not drafted, and is listed in `skipped`
   with a `reason`, when it is: of rule `waiver.matches-nothing` (a waiver for a waiver would hide the
   report that tells the author a waiver does nothing); not tied to a file; or already named by a waiver
   record with the same `Rule` and `Scope` (effective, ineffective or expired). The last makes a second run
   draft nothing, so the command is idempotent and `skipped` names the existing waiver.
4. **Grouping.** One waiver per rule and file: `Rule` is the finding's rule, `Scope` its file path. Several
   findings of one rule in one file share a waiver, because `covers` cannot tell them apart
   (`waiver.rs:180`). `*` is never drafted: a repository-wide scope is a judgment, not a transcription.
5. **The draft is a normal waiver.** It is produced by the same path as `urzua new waiver`: numbering across
   the type's directories fitted to its declared identity, a minted `Stable-Id`, the type's template body, a
   revision-log row, and no overwrite. The scaffold code is shared with `new`, not copied, so the two cannot
   disagree (`ADR-70`). The command carries no waiver field names of its own beyond the ones `Waiver` already
   reads: `Rule`, `Scope`, `Reason`, `Expires`. The body states facts only: the finding messages, verbatim.
6. **It never invents a reason.** `Reason` is the type's pending placeholder. Where the suppression type
   declares `statuses.pending`, the draft takes its first whole-value entry. Because a waiver has no `Status`,
   the build lets a suppression type declare `pending` without `values`. Both gates are relaxed: the
   serde default on `StatusDecl.values` (`status_model.rs:35`) and the empty-list check (`:368`); relaxing
   one alone still fails to load. The build also checks every reader that assumes a non-empty `values`.
   `decision_types` (`config.rs:1597`) keeps working: a values-less type simply lists no status, so a
   `decision_types` entry naming one still gets its clear "does not list" error. The readers it must
   re-examine and cover with tests are `StatusDecl::declares` (`status_model.rs:194`), the iteration at
   `:263` and the per-status loop at `:437`, the initial-status lookup `values.first()` (`commands/new.rs:180`,
   which becomes `None`), the pair check (`config.rs:2937`) and the `status.value-not-declared` message
   (`rules/status.rs:308`). This is the only configuration change this decision needs. Where none is declared
   the draft takes `Pending`, the value the engine already reads as pending for an undeclared vocabulary
   (`field_state.rs:58`), and the report says which source was used (`pending_source`). `Expires`, `Category`
   and `Justified-by` are written `null`, as `new` writes them.
7. **Before writing, the command proves the draft is ineffective.** It loads the suppression type as
   `check` does and classifies the chosen value with the same function (`waiver.rs:219`). It refuses to run
   (exit 2, nothing written, the reason named) when: `Reason` is not in `effective_requires`, since a
   pending `Reason` would then suppress; or the value classifies as `Present`; or the declared `pending`
   holds only patterns and no whole value to write. A pending placeholder that suppressed would be the
   baseline this decision rejects.
8. **`check` then holds the line.** A draft is not effective, so its finding is reported again and
   `waiver.matches-nothing` reports the draft as `waiver-ineffective` naming `Reason` (`rules/waiver.rs:76-107`).
   With `fail_on: warning` the gate stays red until each draft has a real `Reason`. The author adds
   `Expires` and, where the rule has a `permanent` entry, a `Category` and any required `Justified-by`.
9. **It respects `ADR-86`.** The command writes neither `Expires` nor `Category` nor `Justified-by`, so it
   cannot create a permanent waiver or choose a permanence category. Where the type declares `permanent`, a
   draft without `Expires` is refused for every rule (`RuleNotListed`) or lacks its category
   (`CategoryMissing`), so filling in only `Reason` still yields an ineffective waiver until the author
   chooses a date or a listed category. A repository that declares no `permanent` has no such second
   barrier: a waiver without `Expires` is then ungoverned and effective (`waiver.rs:168-170`), so someone
   who fills in only `Reason` creates a permanent waiver by accident, the gap `ADR-86` leaves. Point 7 guards
   only the pending value, so the command also refuses (exit 2, nothing written) when the suppression type
   neither declares `permanent` nor lists `Expires` in `effective_requires`. The message names the fix:
   declare `permanent` (`ADR-86`), or add `Expires` to `effective_requires`. Refusing was chosen over a
   warning because a warned-about draft is still written, and the accidental permanent waiver is the
   outcome to rule out; the repository changes its configuration once and every later draft is safe.
10. **Filters.** `--rule <ID>[,<ID>]` limits to named rules, and an id this build does not ship is an
    error (exit 2), as for `check --rules` (`commands/check.rs:304`). `--path <PATH>` (repeatable) limits to
    findings whose file is that path or lies under that directory; it is a path, not a glob, per `ADR-64`.
    A filter that matches no finding is listed in `unmatched_filters`, never a silent no-op.
    Info findings are skipped: they are below `fail_on`, do not gate, and are not listed in `skipped`, to
    avoid noise; a `--min-severity` field is added only if asked. Decided 2026-11-05: the fallback is the
    compiled-in `Pending` (point 6), `--path` takes plain paths, and one waiver is drafted per rule and file.
11. **Dry-run by default, `--apply` writes.** Without `--apply` nothing is written and the report shows the
    waivers it would write, each path being the number it would take now. With `--apply` it writes, taking a
    number again if another file claimed it first; on any write error it stops and the report lists what was
    written and what was not. `--by` names the author as for `new`.
12. **The report is JSON on stdout**, the only thing on stdout, in the structured style of `ADR-64`:
    `status`, `mode` (`dry-run` or `apply`), `pending_value`, `pending_source`, `drafts`, `skipped` and
    `unmatched_filters`. A draft carries `path`, `stable_id`, `rule`, `scope`, `findings` (their count and
    lines) and `written`. Exit `0` when the command ran, whether or not it drafted anything, and `2` when it could not run.
    It never exits `1`: whether the repository is clean is `check`'s answer.
13. **`migrate schema --assist-waivers` is not built by this.** `SPEC-14` keeps it as named. Its open question
    (does a generated draft preserve the review property) is answered here for `waive`: yes, because the
    draft suppresses nothing until a person writes the reason.

## Amendment (2026-11-08): a finding the drafts themselves resolve is skipped

Status: accepted by the maintainer on 2026-11-08. Point 3 gains one exclusion; nothing else in this
decision changes.

A dry run of an adoption with no waiver directory yet showed the command would draft a waiver for a
finding that exists only because no waiver record exists, a directory that matches nothing. Writing the
first draft makes that finding disappear, so its draft would waive nothing and would be reported at once
by `waiver.matches-nothing`.

Point 3 is amended: before writing, the command judges the tree as it would be with the drafts written,
read from memory and not from disk. A finding that is absent from that judgment is not drafted; it is
listed in `skipped` with the reason that the drafts resolve it. The engine names no rule for this: any
finding the drafts resolve is skipped the same way, and each one is reported, never dropped silently.
`MILE-205` builds it.

## Amendment (2026-11-13): a drift baseline is a git ref, and a git ref is not a second suppression list

*Status of this amendment: decided by the maintainer on 2026-11-13; wording pending maintainer review. `Status` stays `Accepted` and the Decision text above is unchanged.*

Point 1 says there is no baseline file and no second suppression mechanism. `MILE-49` asks for a drift baseline:
a way for a corpus adopting urzua to tell drift that was already there from drift introduced since, so `check`
can gate on the second without the first. Read against point 1 the two look opposed. The maintainer decided on
2026-11-13 that they are not, and chose a git ref in configuration (`embodiment.consistency`'s `drift_baseline:
{ref}`, `SPEC-3`) over a declared list of records. This amendment records why the ref is not what point 1 rejects.

1. **It holds no list.** What point 1 and `ADR-11` refuse is a committed list of today's findings that `check`
   ignores: unreviewable, and growing as the corpus does. A ref names one point in history. Which records are
   pre-existing is computed on every run from the corpus and git, by judging each record as it stood at the ref
   (a record is pre-existing only if it existed there and its drifted locator, with the same pin, had already
   drifted there), so nothing in the configuration enumerates a record or a finding, and nothing can fall out of
   step with the corpus. Editing a record after the ref, to change a pin or to add a citation, does not hide
   drift: the edited or added locator was not drifted at the ref, so it is introduced and reported.
2. **It expires itself.** A list entry outlives its cause until a person deletes it. A ref's effect shrinks as
   records are re-verified and re-pinned, because a re-pin or a re-verification removes the drift, and a record no
   longer drifting is no longer held out. When nothing is held out, a whole-corpus `check` says so in an info notice, the nudge to
   delete the option. Advancing the ref forgets older drift on purpose; deleting it ends the baseline.
3. **It generalises `--base`, which already works this way.** `check --base <ref>` (`MILE-123`) reports only on the
   records changed since the merge base and their dependents, so a pull request is gated on what it touched and
   not on a backlog. That is a baseline whose ref moves with each branch. `drift_baseline` is the same idea with
   the ref fixed in configuration, for the whole-repository run that `--base` does not cover and for the day an
   adopter first runs `check`. It adds no machinery for the question of what is old: it reuses the judgement
   `--base` already makes of a record's copy at a ref (`compute_base_drift`, the drift of the base copy), so the
   baseline is that same comparison with the ref fixed.
4. **Suppression stays the waiver.** The baseline suppresses nothing in the waiver's sense. It does not hide a
   finding of any rule: it states which drift predates a point, the held-out records are listed in an info
   notice and the report's `baseline` object on every run, and a finding that is not drift is untouched. A waiver
   remains the only way to silence a finding, and its `Reason` and `Expires` are not replaced by a ref. A record
   with a locator changed after the ref, a pin edited or renewed after it, or a record added after it is reported
   in full, even where an earlier change had already drifted it.
5. **It cannot fail open.** A ref that does not resolve stops the run (exit 2) instead of reading as no
   pre-existing drift, which would put the whole backlog back on the gate.

What this does not decide: the baseline's scope is drift of `Realized-by` locators under `embodiment.consistency`
alone, and no other rule gains one. A baseline for any other rule's findings is still the waiver, drafted by
`urzua waive --from-report`. A cited file changed since the ref makes the drift of every record citing it
introduced, which is stricter than blaming only the changed lines.

## Reversibility

Easy for the command, which is additive and writes only ordinary waiver records; removing it leaves every
draft a valid waiver. The records it wrote stay as they are. The relaxation of `statuses` to allow `pending`
alone is additive: a type that declares `values` is read as before. The shapes that become hard to reverse are
the report fields and the choice of `Pending` as the fallback value, which a repository can override with a declaration.

## Consequences

- Adoption on a corpus with N findings in F rule and file groups costs F drafts and F reasons, not N
  transcriptions; for the corpus that prompted this the 91 hand-written waivers become one command and 91 edits
  that need judgment.
- No finding is hidden by the tool. Immediately after `--apply`, `check` reports the same findings plus one
  `waiver-ineffective` per draft, so the repository gets worse before better; that is the intent. A repository
  that turns `waiver.matches-nothing` off still sees every original finding, since a draft never suppresses.
- A draft costs the reviewer a file per rule and file, which is the review surface a baseline would have collapsed.
  The reviewer reads real reasons, not a diff to one list.
- The pending placeholder is the type's own vocabulary where declared, so a repository that wants `Reason:
  TODO(owner)` as its marker says so in config. `Pending` is the fallback only.
- A repository whose waiver type does not require `Reason` cannot use the command until it does (point 7).
  The refusal says so; it does not edit the configuration.
- One waiver per rule and file means one `Reason` covers every future finding of that rule in that file,
  while the body lists only today's messages. `covers` cannot tell findings in a file apart, and finer grain
  needs a change to `Scope` that `ADR-86` rejects (no list or glob scope). `ADR-86`'s per-record rule governs
  permanence causes, and for record-scoped rules a file is a record.
- A repository with no `permanent` declaration and no `Expires` in `effective_requires` cannot use the command
  until it declares one (point 9). The refusal names both fixes.
- Demoting a rule to `info` (option C) remains available and is not rejected as a policy: it is rejected as the
  answer to a one-time backlog.
- `SPEC-10` (the draft shape and the pending vocabulary), `SPEC-3` (`pending` on a suppression type), a new
  spec for the command, and `SPEC-12` (the shared scaffold) change in `MILE-205`.

## References

- `ADR-11` -- a waiver is a first-class record, not an ignore list.
- `ADR-86` -- permanent waivers; the draft writes no `Expires`, `Category` or `Justified-by`.
- `ADR-73` -- compatible: it defers a `waive` action kind and adds nothing here. `waive` is not a suggested-action
  kind; the existing `FIELD_SET` action on `waiver-ineffective` (`rust/crates/urzua-core/src/rules/table.rs:1608`) already points at `Reason`.
- `ADR-64` -- structured config in, structured output out; no string mini-language.
- `ADR-70` -- scaffolding goes through one path and carries no schema knowledge of its own.
- `BUG-177` -- a waiver without a real `Reason` suppresses nothing; the property this relies on.
- `MILE-168` -- `waiver.matches-nothing`; `MILE-193` -- the permanence vocabulary.
- `SPEC-10`, `SPEC-12`, `SPEC-14`, `SPEC-3`.
- `MILE-205` -- builds this decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | The milestone that builds this is `MILE-205`, renumbered from `MILE-204` after a numbering collision. | **structural** |
> | 2026-11-05 | Records-only amendment after review (PR #364). Relaxing `statuses` needs both `status_model.rs:35` and `:368`, and the readers of `values` (`config.rs:1597` and others) are named. The command refuses when a repository declares no `permanent` and does not require `Expires` (point 9). Recorded as decided: fall back to `Pending`, plain `--path`, info findings skipped, one waiver per rule and file. `ADR-73` cited as compatible. **Why:** independent review found the serde failure and an accidental-permanent-waiver gap. Status stays Proposed. | **substantive** |
> | 2026-11-06 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-06. | **substantive** |
> | 2026-11-08 | Amended: point 3 skips, and lists in `skipped`, a finding that writing the drafts would itself resolve, judged on the tree with the drafts in memory. **Why:** Maintainer decision, 2026-11-08, after an adoption dry run showed a draft for a finding that exists only because no waiver record does yet. | **substantive** |
> | 2026-11-13 | Amended: a drift baseline declared as a git ref in configuration is not the baseline file of point 1 (no list, expires itself, generalises `--base`). The Decision text is unchanged; `Status` stays `Accepted`. Decided by the maintainer on 2026-11-13; wording pending maintainer review. **Why:** `MILE-49` asks for a drift baseline, and point 1 rejects a baseline file; the maintainer chose the ref form. | **substantive** |
