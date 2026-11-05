---
Stable-Id: 01M6MKSTD2091W0RSX4TQJ8XKT
Status: Accepted
Accepted-on: 2026-11-05
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-7
---
# 78 — RFC-7's remaining enforcement-harness decisions

## Context

`ADR-69`, `ADR-70` and `ADR-71` record the parts of `RFC-7` that Phase 0 builds: the pre-write hook that
relays `urzua`'s verdict, drafting commands, and the layered gate. They left `RFC-7`'s remaining
questions open, and `MILE-12` cannot close while a part of `RFC-7` is neither decided nor deferred with
an owner. The maintainer decided the remainder on 2026-11-05. What is true today:

- **The hook's override is visible only in its output.** `scripts/hooks/record-edit-hook.sh` accepts a
  named override and prints it, then discards it (`persist_override` is an empty function). `ADR-69`'s
  Consequences say that without persistence the override is an audit trail only for as long as the
  transcript lives.
- **The hook judges one computed field.** It relays `urzua fix --stdin-path`, which reports `Embodiment`
  repairs only. A header, field, section or status error in the same edit passes the editor and is
  caught at the commit and in CI (`ADR-71`).
- **`RFC-7` section 4** (scope-mismatch between a branch's stated purpose and its diff) already has a
  rule: `diff.source-and-records-mixed` (`MILE-142`), opt-in, reporting `not-applicable` on a run with
  no `--base`. This repository does not enable it, because it deliberately changes records and code in
  one pull request.
- **`RFC-7` section 5** (prefer the lighter mechanism) has no check. A `Realized-by` locator on a file
  under `back_pointer_paths` is accepted although an `Implements:` or `Verifies:` comment there is
  scanned automatically.
- **A skipped workflow stage** (a decision shipped with no RFC, a change with no ADR) has a record type
  for exactly this kind of recorded exception: the waiver (`ADR-11`).
- **A hook-blocked edit** never reaches the file, so there is nothing to append a revision row to
  (`ADR-14`, `RFC-6`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave the six questions on `RFC-7` | Nothing to build | `MILE-12` stays open; the override stays an audit trail only as long as a transcript |
| B. A notification or routing system for overrides | Someone is told | A service with its own policy and owner; nothing in this repository needs it |
| C. Answer each question with what the tool already has: a committed log read by `check`, one `check` call at the hook, an existing rule, one new opt-in warning, an existing record type (chosen) | No new substrate; every answer is a declared, checkable thing | The hook needs `check --stdin-path`; one rule is new |
| D. A new skip-declaration field for workflow stages | Names the exception exactly | A second exception mechanism beside the waiver, with no computable ground truth to disagree with |

## Decision

In the context of `RFC-7`'s enforcement harness being partly built (`ADR-69` to `ADR-71`), facing six
questions it left open, we decided **the six answers below, each reusing a mechanism the tool already
has**, to achieve an enforcement harness whose overrides are recorded and reviewed and whose editor
layer judges the whole record, accepting that an override is surfaced to a reviewer and not routed to
anyone.

1. **A hook override is recorded in a committed override log.** Each entry carries the override's name,
   the field, the record, the reason and the date. `check` reports an override that has not yet been
   reviewed, so it surfaces in the pull request that carries it. There is no notification or routing
   system: the pull request is the channel. This answers `ADR-69`'s open questions on where an override
   is persisted and whom an unacknowledged block routes to.
2. **The pre-write hook judges every rule on the edited record, not only computed fields.** This needs
   `urzua check --stdin-path`, the prospective-content form of `check` that `fix --stdin-path` is for
   `fix`. An error blocks the edit; a warning is shown and does not. The `ADR-69` behaviour is kept:
   when `urzua` cannot judge, the edit proceeds and the hook says loudly that it did not judge, and a
   block can be passed only with a named override that appears in the output and is recorded by item 1.
   This answers `ADR-69`'s open question on whether other computed fields join `Embodiment`: all of the
   record's rules do.
3. **`RFC-7` section 4 is satisfied by the existing opt-in `diff.source-and-records-mixed`.** No new
   check is built. It stays off in this repository, because this repository deliberately mixes records
   and code in one change; an adopter that wants the prompt enables it.
4. **`RFC-7` section 5 becomes an opt-in warning rule.** For a `Realized-by` locator that names a file
   under `back_pointer_paths`, it warns and nudges toward an `Implements:` or `Verifies:` comment in that
   file. It is a nudge and not an error, because the two mechanisms are not interchangeable in general:
   `Realized-by` is the only option for a path the comment scan cannot reach.
5. **Skipping a workflow stage is expressed with an existing waiver record.** There is no new skip
   mechanism and no `Skip-RFC` field.
6. **A hook-blocked edit leaves no revision record.** This matches `RFC-7`'s lean: a blocked edit never
   happens, as a failed compile leaves no artifact. The override log of item 1 records an edit that was
   let through, not one that was stopped.

With these, every part of `RFC-7` is decided in an ADR or deferred by name, and `MILE-12` closes when
this ADR is accepted.

## Reversibility

Easy for items 3, 4, 5 and 6, which add no data and one opt-in rule. Items 1 and 2 are the commitments: a
committed override log becomes history a reader may cite, and a hook that judges every rule can be
switched off but not made to have judged earlier edits. The cost of being wrong is a hook that blocks too
much, which is overridden often (and recorded as such by item 1), or a log nobody reviews.

## Consequences

- An override stops being an audit trail only for the life of a transcript: it is a committed entry a
  reviewer sees in the pull request.
- `urzua check` gains `--stdin-path` and a finding for an unreviewed override, and the override log
  needs a declared location and shape. `SPEC-2` and `SPEC-8` change when this is built.
- The editor layer now blocks on a header, field, section or status error in the edited record, so the
  hook is only as fast as a single-record `check` run, and the same loud "not judged" output is needed
  for every way `check` cannot run.
- The new rule enters the rule table (`make rule-table-check`) and `SPEC-2`.
- An unreviewed override with no one assigned to review it can sit in the log. The pull request gate
  surfaces it; it does not guarantee a person acts.
- `RFC-7` stays a Draft whose parts are now each decided in `ADR-69` to `ADR-71` or here.
- The milestones that build items 1, 2 and 4 are not filed by this decision.

## Amendment (2026-11-05): the editor hook blocks on errors the edit introduces, and `block_on` may widen that to every error

*Status of this amendment: accepted by the maintainer on 2026-11-05 (drafted 2026-11-05 at the maintainer's request). `Status` stays `Accepted` and the Decision text above is unchanged.*

Item 2 says "an error blocks the edit; a warning is shown and does not". It does not say whose error. The
merged hook (`MILE-191`, #308) reads it as an error the edit introduces: it runs `urzua check --stdin-path` on
the proposed content, runs it again on the record as it is on disk, and blocks only on an unwaived error whose
rule and message the on-disk record does not already report (`scripts/hooks/record-edit-hook.sh:95-107`, the
`introduced` and `existing` sets, and the exit at `:125-130` when `introduced` is empty). An error already on
disk is listed under "Already failing on disk before this edit (not blocked)" (`:122`). A baseline that cannot
be judged leaves every error introduced (`:96`, `SPEC-8`). `MILE-191`'s Done means was trimmed in #305 to say
"an unwaived error finding on the edited record", with the pre-existing allowance removed on the ground that no
ADR sets it, so the record and the code disagree. This amendment is that ADR text, and it adds one option.

**1. The default is `introduced`, which is what the hook does today.** An edit to a record that already has an
error is not refused for that error, so the edit that fixes something else, or adds a revision row, goes
through. The error is still shown. This is item 2's "an error blocks the edit" read as "an error the edit
introduces blocks it".

**2. A hook option, `block_on: introduced | all`.** `all` blocks on any unwaived error in the proposed content
of the edited record, whether or not the record already had it; the existing-error set is then empty and the
second `check` run is skipped, which makes a blocked edit faster. Any other value is refused. The named override
(item 2) and its log (item 1) work the same under both values. The option is read by the hook from the
configuration the way `override_log` is (a top-level scalar, `record-edit-hook.sh:136-139`), so `introduced`
is also what a repository gets when the key is absent. Its exact key name and placement are for the build
(`MILE-195`) and `SPEC-3`; `block_on` is the working name.

| | `introduced` (default) | `all` |
|---|---|---|
| Pros | An edit is judged on what it did: an agent is not blocked by an error it did not write and may not be able to fix in this edit; a record with a legacy error can still be amended; fewer overrides, so the override log stays meaningful | A record is never written in a failing state; the error cannot sit unnoticed behind unrelated edits; one `check` run per edit, no baseline comparison, no same-rule-and-message matching to get wrong (two errors with one message count as one existing) |
| Cons | A failing record can be edited indefinitely and is caught only at the commit and in CI (`ADR-71`); the "introduced" test is by rule and message, so a changed line number or a second identical finding is invisible; two `check` runs on an edit that has errors (about 0.4s more) | An edit that does not touch a failing section is refused until the error is fixed, so a large record with one error cannot take a revision row; an agent must fix unrelated errors first or use a named override, and overrides pile up in the log; adopting the hook on an existing corpus with errors blocks every edit to them |

**3. This repository stays on `introduced`.** CI runs `urzua check` with `fail_on: warning` and holds the
repository at zero findings, so a record on disk with an error does not reach `main`; the case `all` guards
(an error already present when an edit begins) is rare here and is caught at the commit. `all` is for a
repository that has no such gate or wants the stricter editor layer.

**4. Nothing else in the decision changes.** Items 1 and 3 to 6 are untouched, the "not judged" behaviour of
item 2 is unchanged, and a hook-blocked edit still leaves no revision record (item 6).

The build is `MILE-195`. `MILE-191`'s Done means states the allowance again as the default behaviour, with a
revision row.

## Amendment (2026-11-05): after any Bash command the harness runs `check` on the records git shows as changed

*Status of this amendment: proposed, pending maintainer acceptance (decided by the maintainer on 2026-11-05, drafted with `ADR-89`). `Status` stays `Accepted` and the Decision text above is unchanged.*

Item 2 puts the judgement of a record at the editor. The editor hook sees three tools: its matcher is
`Write|Edit|MultiEdit` (`.claude/settings.json:5`) and its adapter returns for any other tool
(`scripts/hooks/claude-code-record-edit.py:55`). A record written or changed by a Bash command (`sed -i`, a
heredoc, `tee`, `cp`, an interpreter one-liner) is not judged there, so `ADR-71`'s "one gate at the editor"
holds for three tools and not for the fourth. This amendment adds a check after the command, and does not
change item 2.

**1. After any Bash command, `check` runs on the records git shows as changed.** The harness lists the changed
paths from git, keeps the ones a record type governs (the engine's own discovery decides, not the hook), and
runs `urzua check` on exactly those paths, then reports the unwaived findings to the agent as context. It
does not read the command: no parsing of shell, no list of write verbs, no guess about which files a script
touched. Git's view of the working tree is the ground truth and a Bash command that changes a record by any
route changes it.

**2. The list includes untracked files, and `check PATHS` examines them.** The merge-base diff `check --base`
uses is `git diff --name-status` (`rust/crates/urzua-io/src/lib.rs:179-189`) and lists tracked changes only, so
a record a Bash command creates is not in `--base`'s changed set. It is not missing from `check PATHS`, which
examines a file named on the command line whether or not git tracks it (`resolve_argv_overrides`,
`rust/crates/urzua-cli/src/discovery.rs:321`, `BUG-24`; `rust/crates/urzua-cli/src/commands/check.rs:446-470`).
The one refusal is `--base` combined with a named untracked file (`check.rs:451-456`), and the post-command
check does not use `--base`. So it takes its paths from `git status --porcelain -z`, which names modified,
added and untracked files, keeps the governed ones and runs `urzua check PATHS` on all of them in one call,
tracked or not, and it does not stage anything. No `--stdin-path` relay is needed. A deleted record is not
judged.

**3. It reports; it does not block.** The command has already run, so the check cannot refuse it. It
reports each finding with its file, rule and message, says which are errors, and says that pre-commit and CI
remain the hard gate (`ADR-71`). `block_on` (above) does not apply, since nothing is being written. A record
that was already failing before the command is listed as it is today, under the pre-existing note, by comparing
with the copy at `HEAD`. A repeat of an unchanged record's findings is not re-reported on later commands:
the report is keyed by the record's content hash, kept under `.urzua/cache`.

**4. It cannot judge a command it did not see, and says so.** When `git` is absent, the working tree is not
a repository, or `check` cannot run, the report says the command's effect on records was NOT judged, the same
loud "not judged" form as item 2 (`ADR-71`); a missing check is never a quiet pass.

| | Pre-command matcher on Bash (rejected) | Post-command check on git state (chosen) |
|---|---|---|
| Pros | Could refuse the write | Needs no knowledge of what the command does; cannot be evaded by a different spelling; one rule for every route |
| Cons | Parsing shell is fuzzy (variables, heredocs, `python -c`); a miss reads as clearance; it needs a list of verbs that grows | Reports after the fact; the record is already changed on disk |

A record that a Bash command moved to an accepting status is reported with the same finding the acceptance
guardrail of `ADR-89` gives an editor edit, as an error-level report in its own right. It cannot be prevented
here, and neither the commit hook nor CI can tell who made the move on a repository with a single maintainer;
`ADR-89` lists this under "Accepted risk" and does not claim otherwise.

The build is `MILE-202`. Switching the post-command check off is a per-guardrail option in `ADR-89`'s
`harness` configuration, on only when named.

## References

- `RFC-7` -- the enforcement harness; sections 2 to 6 and its open questions.
- `ADR-69`, `ADR-70`, `ADR-71` -- the parts already decided, which this builds on without changing.
- `ADR-11` -- the waiver record that expresses a skipped stage.
- `ADR-14` -- the append-only revision log, which a blocked edit never reaches.
- `MILE-12` -- closes when this ADR is accepted.
- `MILE-142` -- `diff.source-and-records-mixed`.
- `SPEC-2`, `SPEC-8` -- `check` and `fix`, where `--stdin-path` is specified.
- `BUG-24` -- `check PATHS` examines a named file git does not track.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-05, recorded here; the maintainer accepts it in a later change. | **substantive** |
> | 2026-11-05 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-05 | Amendment added: the editor hook blocks on errors the edit introduces by default, and a `block_on: introduced \| all` option widens it to every error in the edited record. **Why:** `MILE-191` (#308) merged with the introduced-only behaviour, #305 trimmed `MILE-191`'s Done means, and no ADR sets either; proposed, pending maintainer acceptance. `Status` stays `Accepted` and the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Amendment (`block_on`) accepted, together with `MILE-191`'s restored Done means. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-05 | Amendment added: after any Bash command the harness runs `check` on the records git shows as changed, untracked included, and reports the findings to the agent without parsing the command; it reports and does not block, and pre-commit and CI stay the hard gate. **Why:** the editor hook's matcher does not cover Bash (`.claude/settings.json:5`), so a record written by a shell command is never judged at the editor; maintainer decision, 2026-11-05; proposed, pending maintainer acceptance. `Status` stays `Accepted` and the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Post-command amendment corrected after review: untracked records are examined by `check PATHS` directly (`BUG-24`), so the `--stdin-path` relay is dropped and all changed paths go to one `check PATHS` call; a gated move by Bash is reported as an error and is no longer said to be stopped by the commit hook and CI. **Why:** Independent review of pull request 355 found the untracked-file claim false (`check.rs:446-470`) and the commit-hook and CI claim untrue for a single maintainer (`ADR-89` "Accepted risk"); proposed, pending maintainer acceptance, `Status` stays `Accepted` and the Decision text is unchanged. | **substantive** |
