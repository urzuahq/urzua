---
Status: InProgress
Stable-Id: 01M2W199SQBR59QQ77A6FR11K5
Phase: '1'
Track: governance-process
Implements: SPEC-1
Depends-on: MILE-177
Blocked-on: —
---
# 103 — `urzua new` is the faster path, so hand-authoring stops

## What

`SPEC-1`'s fourth success criterion:

> `urzua new` is what people and agents actually invoke, because hand-authoring is now the slower path.

## Why it is not satisfied by `urzua new` existing

It does exist (`ADR-27`) and is used in this repository. The criterion is about **preference under
choice**, not availability -- it is met when hand-authoring is slower, not when the command is
present.

Evidence it is not met yet: several records in this corpus were hand-written during this project's own
work, and two of them were rejected by `check` for header shapes `urzua new` would have produced
correctly. The command was available and not reached for.

## A friction point found in this project's own use

`urzua new` writes a record and leaves it untracked, and `check` governs tracked files only
(`ADR-56`). Between the two, a freshly created record is invisible: a reference to it from an
already-tracked file is reported as dangling, at `error` severity, blocking, while the record sits on
disk. It happened twice in one session and was misdiagnosed both times as a stale build.

Staging what `urzua new` creates would close that window at the source. Recorded here rather than
decided, because it is a question about which path is faster -- this milestone's subject -- and not
about what is governed, which `ADR-56` settles. It should be configurable if it is built: a command
that touches the index is not what every workflow wants.

## What would satisfy it

A measurement rather than an assertion. Candidates, unchosen:

- **Every record created in a window came from `urzua new`**, checkable from git history against the
  synthesized-header shape the command emits.
- **`urzua new` covers the cases people currently hand-write around** -- it refuses rather than
  guessing when neither a template nor `required_fields` applies (`ADR-27`), and each refusal is
  someone about to hand-author.

The second is the useful one, because a refusal is a recorded instance of the criterion failing.

## Not blocked

Independent of `MILE-98` and `MILE-100`. It is about the authoring path in whatever corpus is in
front of the tool.

## Done means

- Records in this repository are created through `urzua new` and not by hand: `urzua new` is measurably the faster path (a recorded comparison of time or steps against hand-authoring).
- The agent commands of `MILE-177` route through `urzua new`. — evidence: test:rust/crates/urzua-cli/tests/drafting_commands_integration.rs#each_command_drafts_a_record_that_check_examined_and_passes@sha256:cc8b6be8e02f7d3f70bf60e503be3fa691cecc2769d77eb4eab9ada2be8c503a, code:.claude/hooks/urzua-record.sh@sha256:6b6e8133b8d92377ca30e48f80357319b26afdac6cc71e79b442b161b6a7c3de
- Over a stated window of at least 30 calendar days and at least 50 new records, no record is merged whose header differs from the scaffold `urzua new` would have written and that a header rule rejected at a pull-request head; the count is taken with `scripts/replay-header-comparator.py` (the opt-in `header.scaffold-mismatch` rule over the commits of each merged pull request), and the window, the number of records and the count are recorded here when it is met (`SPEC-1` Phase 1 exit criterion).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s criterion 4 had no milestone and was deleted from the spec alongside criterion 3. It is about preference under choice, not availability -- `urzua new` exists and records in this corpus were still hand-written, two of them rejected for header shapes the command would have produced correctly. | **substantive** |
> | 2026-11-04 | `Phase` 0 -> 1; `Status` unchanged. **Why:** Maintainer decision, 2026-11-04: `urzua new` being the faster path is a Phase 1 exit criterion, not part of the Phase 0 exit; `SPEC-1` already lists it under Phase 1. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Depends-on: MILE-177` added. **Why:** Done means requires the agent commands of `MILE-177` to route through `urzua new` (Done). Proposed for maintainer review; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-14 | Done means item 3 reworded to say what is counted and where: a window of at least 30 calendar days and at least 50 new records, "differs from the scaffold `urzua new` would have written" and "a header rule rejected at a pull-request head", the count taken with `scripts/replay-header-comparator.py`. Item 2 gains evidence (the drafting-commands test and the shared hook), which the work of `MILE-177` already delivered. `Status` is set by the engine below. **Why:** maintainer decision of 2026-11-13: the merged state always passes `check`, so "rejected" is observable only at a pull-request head, and "the header `urzua new` would have produced" depends on the configuration at the creation commit; the wording named neither. | **substantive** |
> | 2026-11-14 | Instrument built: the opt-in rule `header.scaffold-mismatch` (`SPEC-2`), which under `check --base` compares each added record's header with the one the shared, pure `scaffold_record` renders from the run's configuration and the type's template, taking the record's own number, title, id, author and date; a null field may be filled in or left out and nothing else may differ. It is declared at `warn` in this repository's `.urzua/config.yaml`. `scripts/replay-header-comparator.py` replays it at every commit of each merged pull request that adds a record, with that commit's own configuration, and over the merged commits themselves; `scripts/time-new-vs-hand.py` measures item 1. No provenance stamp (decision of 2026-11-13). **Why:** the maintainer's plan of 2026-11-13: a comparator and a replay, not a stamp. | **structural** |
> | 2026-11-14 | Replay over the 30 days before the instrument existed (2026-10-15 to 2026-11-14, `origin/main` at `3f4186a5`; 374 commits on main, 355 merged pull requests, 179 pull-request-head commits replayed). Records added and examined: 243. Of those, 30 (12%) differ from the scaffold, for two reasons: 17 carry a field the type's template does not list (13 milestones with `Depends-on` or `Implements`, 3 specs with `Implements` or `Parent`, 1 ADR with `Non-reference-ids`; see `MILE-211`), and 13 were created already decided (6 bugs `Fixed`, 4 milestones `Done`, 3 of them with `Realized-by` added, and 3 ADRs `Accepted` with `Accepted-on` added). A null field left out is not counted. At pull-request heads, 31 (pull request, record) pairs differed at some head and 28 still did at the head that merged. **Rejected by a header rule while differing: 1**, ADR-99 in pull request 492 (`Accepted-on` added and `Status` set to `Accepted`, reported by `status.transition-allowed`; conformant at the final head). Target 0 is not met on this baseline. No commit failed to run: the configurations of the 92 oldest replayed commits name rules this build does not ship, so `header.deprecated-shape` and `header.layout-consistency` (92 commits), `date.field-order` (5) and `citation.path-exists` (4) were dropped from them for the replay. **Why:** the baseline the window will be read against; the window itself has not started. | **structural** |
> | 2026-11-14 | Comparison of the two ways of writing a record, `scripts/time-new-vs-hand.py --runs 3`, five types (adr, bug, milestone, rfc, spec), 15 runs per arm, every run ending at a record `urzua check` accepts. Steps (one action an agent takes with a tool): 5 with `urzua new` (scaffold, read the scaffold, read the config, fill, finish) against 9 by hand (read the config, read the template, list the directory, read the newest record, mint an id, write, stage, index, check). Text the author has to write, in bytes: 15 (adr), 17 (bug), 124 (milestone), 0 (rfc), 0 (spec) with `urzua new`, against 1159, 564, 681, 915 and 875 by hand. Time of the commands themselves, mean of 3: `urzua new` is slower, 3.7 to 4.1 s against 2.3 to 2.8 s as configured here (its scaffold step takes about 1.4 s because `new.check_open_prs` fetches pull-request heads, a safety check the hand path does not have) and 2.7 to 3.4 s against 2.1 to 2.7 s with the fetch off; `urzua check` is about 2 s of either path. The hand arm is a floor: it is scripted, writes the same text the new arm ends with and makes no mistake, so it pays for no failed check; model time is not measured. So `urzua new` is the faster path in steps and in what the author must write, and not in command time. Item 1 stays unresolved until the maintainer decides whether that is what "measurably the faster path" asks for. **Why:** the maintainer's plan of 2026-11-13, item 4 (5 types x 3 runs). | **structural** |
> | 2026-11-14 | Window not started. The 30-day window starts at the commit that merges the change adding `header.scaffold-mismatch` to this repository's configuration, and ends 30 calendar days later. It needs, in that window, at least 50 records created (the 30 days before saw 243), then `python3 scripts/replay-header-comparator.py --since <start> --until <end>` (pull-request heads fetched first, see the script) reporting 0 rejected pairs; the start, the end, the number of records and the count are then recorded here and the item resolved. `Status` stays `InProgress`; the milestone is not `Done`. **Why:** the window cannot complete inside one pull request. | **structural** |
> | 2026-11-14 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (1 of 3 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
