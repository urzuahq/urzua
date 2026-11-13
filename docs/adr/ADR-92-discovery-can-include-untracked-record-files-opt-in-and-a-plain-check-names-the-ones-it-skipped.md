---
Stable-Id: 01M6SK9J9JPGS00G7E3FHKRCGC
Status: Accepted
Accepted-on: 2026-11-06
Date: 2026-11-06
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 92 — Discovery can include untracked record files, opt in, and a plain check names the ones it skipped

## Context

Line references are to `origin/main` at `cdca1179`. `lib.rs` is `rust/crates/urzua-io/src/lib.rs`;
`discovery.rs`, `commands/check.rs` and `harness/post_command.rs` are under `rust/crates/urzua-cli/src`.

`check` governs the files git tracks. `ADR-56` kept that on purpose: "untracked-means-ungoverned
stands unchanged", because a scratch file in a working tree is not part of the corpus. The maintainer
decided on 2026-11-06 that a repository must be able to choose the other way, because a record drafted
and not yet staged is exactly the record an author most needs `check` to read, and that nothing about the
default may become silent.

What exists today:

- **Discovery is `git ls-files` plus staged files.** `discover_tracked_files` runs `ls-files -z`
  (`lib.rs:77`) and `diff --name-status --cached -z` (`lib.rs:88`) and unions them. It lists no untracked
  file, and the doc comment at `lib.rs:68-70` makes that a contract ("never a raw filesystem walk").
  Two other callers use the same function and must see the same set: `file_texts.rs:34` and
  `session.rs:79` (and `commands/init.rs:504`).
- **A file named on argv is the one exception, and only a file.** `resolve_argv_overrides`
  (`discovery.rs:337-359`) adds a named path that is a regular file and not tracked; a directory argument
  still respects the tracked sweep (comment at `discovery.rs:333-336`). `open_session` (`commands/check.rs:530-571`)
  then marks the source `Argv` (`:561`). This is `BUG-24`; its second pinned test,
  `an_untracked_file_below_an_explicitly_requested_directory_is_not_examined`, asserts the directory case.
- **`--base` cannot take an untracked file.** `changed_paths_since` diffs the merge base against the working
  tree with `git diff --name-status --no-renames -z` (`lib.rs:190-199`), which lists tracked paths only; its
  doc says "Untracked files are not reported" (`lib.rs:152`). `open_session` therefore refuses a named
  untracked file under `--base` (`commands/check.rs:545-556`) instead of dropping it.
- **The tool already knows which untracked records exist and says so only on failure.**
  `untracked_records` (`discovery.rs:184-223`) reads the direct children of every declared record directory
  that are not tracked and loads them as records. `check` builds it on every run (`commands/check.rs:874`)
  and three rules use it only to word a failure: `pointer-untracked` (`rules/pointer.rs:141`),
  `blocker-untracked` (`rules/sequence.rs:371`) and `locator-untracked` (`rules/embodiment.rs:230`). A plain
  `check` that skips three drafted records prints nothing about them.
- **Drift needs history, which an untracked file lacks.** `commit_for_line_in` answers `Ok(None)` for "no
  history for the file or line (untracked, staged-new, unborn HEAD)" (`lib.rs:619-624`, `is_no_history_stderr`
  at `:763`), and the drift loop treats that as nothing to judge (`discovery.rs:615-620`, `_ => continue`).
  Today that is silent.
- **The harness already reads untracked records by name.** `post_command.rs:6-7` sends modified, added and
  untracked governed files to one `check PATHS` call, so the editor path is covered and the whole-repo gate
  is the gap.
- **Where it bit.** `BUG-24` was found filing a record and running `urzua check` on it: `files_examined: 0`,
  no findings, until `git add`. `ADR-56` recorded two further occasions where the tool's own scaffolding
  left a record untracked.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave as is (`ADR-56`) | Nothing to build; drafting stays free | A plain `check` over a tree with three drafted records reports a clean run that did not read them; the notice that `ADR-55` asks for is absent |
| B. Always include untracked, non-ignored record files | No config; no gap | Reverses `ADR-56`, `ADR-6`'s "never a raw walk" and the pinned scratch-file test for every adopter; governs scratch and half-written files, so the tool becomes hostile to drafting; CI and local runs differ without anyone choosing it |
| C. Name the skipped records in a notice, and include them only when the repository opts in (chosen) | Default is unchanged and no longer silent; a repository that wants drafts governed says so once, in committed config, so every checkout and CI agree | One config key; a second discovery path to test; drift and `--base` need stated behavior for a file with no history |
| D. A `--include-untracked` flag on `check` instead of config | No schema change | Per run, so the hook, CI and a person each have to remember it; two runs of the same tree disagree; config is how every other governance choice is declared (`ADR-53`) |
| E. `urzua new` stages what it writes (`ADR-56`'s fourth option, `MILE-103`) | Removes the window at its source | Covers only records made by `new`, not hand-authored ones; gives a scaffolding command a side effect on the index |

## Decision

*In the context of a tool that governs tracked files and an author who most needs a draft read before it
is staged, facing a maintainer decision that a repository may choose to govern untracked records and that
the default must not hide them, we decided that `discovery.include_untracked: true` makes discovery include
untracked, non-ignored files under declared record directories, and that without it a plain `check` reports
an info notice listing the untracked records it skipped, to make drafting visible without making it
governed by default, accepting a second discovery path and a stated limit on drift.*

1. **The option.** A top-level `discovery` object with one key, `include_untracked`, a boolean, default
   `false`. It is declared in config, not on the command line (option D). Structured, per `ADR-64`; no
   string value.
2. **What is included.** With the option on, discovery adds `git ls-files --others --exclude-standard -z`,
   restricted to the declared directories of every record type (`all_dirs`) and to files the type owns
   (`owns_dir`, the filename rules `load_records` applies). Plain git, no libgit2 (`ADR-6`).
   "Non-ignored" is git's own rule: `.gitignore`, `.git/info/exclude` and the global excludes. A file outside
   every declared record directory is never added; an untracked file a `Realized-by` locator names stays a
   `locator-untracked` finding.
3. **One function, with a new signature and contract.** The untracked set joins `discover_tracked_files`'
   result so `file_texts.rs:34`, `session.rs:79` and `init.rs:504` cannot see a different corpus from
   `check`. Today the function takes no config and its doc comment (`lib.rs:68-70`) forbids this; it now
   gains config input (the option and the declared record directories and ownership rules), so its signature
   changes, every caller is updated, and its doc comment is rewritten: tracked and staged files, plus
   untracked non-ignored record files when the option is on, and still never a raw filesystem walk. The scope source stays
   `GitTracked`; the report's `scope` gains an always-present boolean `include_untracked` and the count of
   untracked records included (`untracked_included`), so a consumer can tell the two corpora apart.
4. **Without the option, nothing is silent.** A plain `check` emits one info notice
   (`discovery.untracked-skipped`) naming each untracked record under a declared directory it did not
   examine, up to 20 paths and a count, built from the same scan as `untracked_records`
   (`discovery.rs:184`). It is emitted only when there is something to list, and not for a record the run was
   told to examine by name (`BUG-24`). Info does not gate, so `fail_on: warning` is unaffected.
5. **Explicit paths.** A named file keeps its `BUG-24` behavior whatever the option. With the option on, a
   directory argument examines the untracked records under it, because the sweep it scopes includes them;
   with it off the pinned directory test still holds. The `--base` refusal for a named untracked file
   (`commands/check.rs:545-556`) applies only when the file is outside what the option would include.
6. **`--base` treats an untracked record as added.** With the option on, `changed_paths_since` unions the
   same `ls-files --others` listing into `changed` and `added`. An untracked record has no merge-base copy, so
   it is new by definition, and `load_base_text` already gives a new record no entry
   (`commands/check.rs:105-106`). The rules that judge a diff (`ADR-90`'s `scope: changed`) therefore judge a
   draft the way they judge a staged addition. In CI there are no untracked files, so a local run is stricter
   than CI only by judging drafts CI will meet after the push. Without the option, behavior is unchanged.
7. **Drift does not judge an untracked record, and says so.** A record with no history has no date for its
   `Realized-by` line, so "a cited file changed since" has no answer. The drift loop keeps skipping it
   (`discovery.rs:615-620`) and the run adds an info notice (`discovery.untracked-not-drift-judged`) with the
   count of included untracked records that carry a locator, so the skip is disclosed and not inferred. A
   record that is staged but new is in the same position today (`lib.rs:621-622`) and the notice counts it
   too, since it is the same fact. A record becomes judged by drift once it has a commit.
8. **This repository turns it on.** `.urzua/config.yaml` sets `discovery.include_untracked: true`, and the
   change is exercised in the repository's own corpus (`MILE-206`).
9. **`ADR-56` stands for the default, and gains an amendment.** Its decision (untracked is ungoverned and
   the diagnostic names the file) is the behavior of a repository that does not opt in. This decision adds
   the opt-in; it does not reverse `ADR-56`. But `ADR-56` says "untracked-means-ungoverned stands unchanged"
   and "nothing about discovery ... moves", and this decision makes both conditional and adds a default
   notice, so a revision row alone is not enough. `ADR-56` therefore carries a dated
   `## Amendment (2026-11-06)` recording that untracked-means-ungoverned is now the default only, in the
   form of the `ADR-27` and `ADR-74` amendments. It is proposed, pending maintainer acceptance, and
   `ADR-56`'s `Status` stays `Accepted`. This is a requirement of this decision, not an option.

## Reversibility

Easy for the option and the notice: both are additive, a repository that never sets the key sees one info
line, and removing the key restores today's corpus. Harder is the report shape: `scope` gains two fields
that consumers may come to read, and the notice subjects are names a repository can waive or filter on.

## Consequences

- A repository that opts in is checked on drafts, so a malformed record is found before `git add`, and
  `files_examined` rises by the number of untracked records. A scratch file in a record directory that looks
  like a record is governed, and the author must move it, ignore it or finish it. That is the price of the
  option and the reason it is off by default.
- A repository that does not opt in learns once per run what it skipped. The notice is the cost of
  `ADR-55`: a tool that knows a record exists must not report as if it did not.
- The three `*-untracked` finding variants stay for records outside the option, and cannot fire for an
  included record, which now resolves. Their tests keep a case with the option off.
- Drift on a draft is a known blind spot until its first commit; the first commit makes it judged. A
  reviewer sees the blind spot in the notice, not by reading code.
- `--base` plus an untracked draft stops being an error when the draft is under a declared directory, so the
  `BUG-24` refusal narrows rather than disappears.
- Two checkouts of one commit with different untracked files disagree on `files_examined`. This is true of
  every local run already (staged files) and is why the key lives in committed config.
- `SPEC-2` (Discovery, the `scope` fields, the two notices), `SPEC-3` (the `discovery` key) and the rule or
  notice tables change in `MILE-206`.

## References

- `ADR-56` -- how an untracked record-shaped file is surfaced; this adds the opt-in and the default notice, and `ADR-56`'s `## Amendment (2026-11-06)` records the effect (clause 9).
- `ADR-6` -- shell out to git; `ls-files --others` is the same dependency.
- `ADR-55` -- the tool says what it knows; the notice is that obligation applied to discovery.
- `ADR-64` -- structured config in, structured output out.
- `ADR-90` -- `scope: changed`, the diff rules that judge an untracked record as added.
- `ADR-53` -- governance is the repository's configuration, hence a config key and not a flag.
- `BUG-24` -- explicit argv paths and untracked records; the pinned directory test.
- `MILE-103` -- `urzua new` staging what it writes, left open by `ADR-56`.
- `MILE-206` -- builds this decision.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-06 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-06: an opt-in `discovery.include_untracked`, a plain `check` that names the untracked records it skips, `--base` treating them as added, and drift not judging them (disclosed). The maintainer accepts it in a later change. | **substantive** |
> | 2026-11-06 | Clause 3 states that `discover_tracked_files` changes signature and contract (it gains config input) and its doc comment is rewritten; clause 9 requires the `ADR-56` amendment instead of leaving it to the maintainer; `ADR-56` cited. Both records stay Proposed. **Why:** independent review of 2026-11-06 found the function takes no config and its doc forbids this, and that `ADR-56`'s "stands unchanged" needs a dated amendment, as `ADR-27`'s precedent shows. | **substantive** |
> | 2026-11-06 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-06. | **substantive** |
