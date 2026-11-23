---
Stable-Id: 01M83CRMXBVEABZM1S2VM9CYDF
Status: Draft
Date: 2026-11-23
Author: beauwilliams
Supersedes / Superseded-by:
---
# 50 — Polyrepo support: each repository keeps its own config, and a record can cite a record in a sibling repository

## Summary
Support organisations that keep their decision records in several repositories, without merging
their configurations. Each repository keeps its own `.urzua/config.yaml` and rules. Two
capabilities follow. First, a record can cite a record in another repository by a qualified
reference (`<repo>:<id>` or a `stable_id`), resolved offline against a committed snapshot of that
repository's `urzua export`, and counted eligible and unexamined when the snapshot is missing or
unusable. Second, an organisation view is built from several repositories' exports, each read in
the vocabulary its own engine produced and labelled by repository; `urzua board` may show it. The
second capability reverses part of `RFC-27`'s amendment of 2026-11-08. This RFC says so and
proposes how to record it. It decides nothing. Several choices are left to the maintainer below.

## Motivation
The maintainer's direction, given 2026-11-23 and relayed from the organisation's coordination
thread. The first point quotes the relayed words; the rest paraphrase the relayed direction:

- urzua supports polyrepo whatever way one organisation arranges its repositories: "there's a lot
  of companies that have this problem".
- Configs never merge, each repository keeps its own config and rules, and there is no org-level
  config governing several repositories.
- The org view is built from each repository's `urzua export`, in the fixed vocabulary the engine
  already produces, and a cross-repository view in `urzua board` is acceptable.
- A record citing a record or schema in another repository is the one new engine capability.
- `RFC-27` is settled explicitly.

This repository's own records show the gap:

- `MILE-358` (line 19) has to say "a gate tool", and `MILE-359` (line 38) "the published gate-report
  JSON Schema", for work defined in another repository. Neither record can cite that work, so nothing
  checks that it exists, what state it is in, or whether it changed after these milestones were
  filed.
- `RFC-11` lists "Cross-repo reference resolution. Roadmap Phase 4" as a non-goal (line 381), and
  `SPEC-3` defers it as well. Its open question on back-pointer patterns (line 2018) says fixing them
  "is what makes cross-repo resolution possible". No record has taken this up since.
- `MILE-24` (Planned, Phase 4) holds "a cross-repo decision graph" and "an org rollup". `RFC-27`
  §3 and its 2026-11-08 amendment class both as ecosystem.
- Outside this repository, three sibling repositories this organisation runs carry urzua records,
  each governed by its own configuration and release. As reported by the organisation coordination
  thread's survey of 2026-11-23 (not checkable from this repository), all four repositories give
  every record a `stable_id` with no duplicate inside any one of them. Nothing checks uniqueness
  across them.

## Proposal

### 1. Each repository stays governed by its own configuration

No configuration is merged, inherited or overlaid. A repository's `check` reads only its own
`.urzua/config.yaml`. Another repository's records enter only as data, in the shape its own
release's `urzua export` wrote, and never as rules.

### 2. A declared list of sibling repositories

The repository's configuration declares a list of siblings it may cite. Each entry has a name (the
qualifier used in references), the forge identity (`owner/name`, for the snapshot check below), and
one source:

- **snapshot** (proposed default): a committed file holding that sibling's `urzua export --format
  json` output plus an envelope `{schema_version, repository, generated_at, urzua_version,
  records}`. This mirrors the issue snapshot `citation.issue-exists` already uses (`SPEC-3`,
  "Cited issues and pull requests", lines 782-835): it is written by a refresh command, committed,
  checked for `schema_version` and repository, and its `generated_at` appears on every finding. CI
  then never needs the network or a second checkout.
- **checkout**: a local path to a sibling working tree, whose export is computed by running that
  sibling's configuration. It suits a developer with sibling repositories checked out side by side.
  CI has no such tree unless it clones one, and cloning needs the network and a token (see open
  question 3).

Nothing reaches a forge or a service. `RFC-14` already judged (line 128) that "a cross-repo graph has
no natural forge surface". A declared sibling whose source is absent is reported as "declared, not
read". It is never skipped silently.

### 3. Qualified references

A reference to another repository's record is written `<repo>:<id>` (for example `<repo>:RFC-2`) or
as a bare `stable_id`. Display ids collide across repositories: several of the four repositories each
have an `RFC-1` (as reported by the 2026-11-23 survey). An unqualified display id therefore always means a record in this repository,
as it does today. In the snapshot, a qualified reference resolves to the target's `stable_id`, and
that `stable_id` is the key the engine stores. The display id is what a person reads. Once resolved:

- `pointer.resolution` reports the target's existence and status, as it does for a local target.
- `narrative-field.stale` fires once the target reaches a terminal state in its own repository's
  vocabulary (`finished` or `dropped`, section 5).
- A file in a sibling is cited `<repo>:path` and checked by `citation.path-exists` against the
  snapshot's list of paths, or against the checkout.

A reference is unexamined, and counted as eligible and unexamined, when the sibling is undeclared,
its snapshot is absent, unparsable, of another `schema_version` or of another repository, or the
target is not in the snapshot. In each case one finding names the reason. The report keeps the
existing split of examined against found (`SPEC-1`: no silent no-op), so a check whose snapshot
never loaded cannot pass as one that found nothing.

`external_ids` (`SPEC-3`, line 644) stays as it is. It names ids that are deliberately not records
anywhere urzua reads (an IETF RFC number, for example), which neither resolve nor report. A qualified
reference is the opposite case: an id that is a record and should resolve. A repository moving an
id from `external_ids` to a qualified reference gains a check and loses nothing.

### 4. The org view

An org view reads N exports. Each export comes from its own repository, under its own config and
release. The view labels every record with its repository, keys it on `(repository, stable_id)`,
and shows the qualified references between records as edges. It computes no state of its own. It
shows `state` as each export gives it, grouped by repository and then by state. It never
recomputes `state` from raw statuses with a merged vocabulary. `urzua board` may render this view
(the maintainer's direction). Section 7 gives the scope test that admits it.

### 5. Finding 1: `state` is not comparable across repositories as-is

`StateCategory` (`rust/crates/urzua-core/src/view_model.rs:25-46`, `state_of` at 218-259) gives
`open` to a type that declares no `progress` list and `none` to a type with no statuses. In this
repository's export of 2026-11-23, 28 of 762 records are `open` (20 rfc, 4 condition, 3 bug, 1 adr)
and 16 are `none` (all waivers). The 2026-11-23 survey reports that in two sibling repositories every record is `open` (3 of 3 and
1 of 1), and that in the third 7 records are `open` (3 rfc records at `Discussion` and 4 spec records at
`Draft`, types with no `progress` list) while its other types sort properly.
In the view, `finished`, `dropped`, `under-way` and `not-started` mean the same thing in every
repository, because each export derived them from that repository's declared lists. `open` means
"nothing declared says whether work began", and that is not `not-started`.

The proposal: an org view shows `open` and `none` as their own columns, labelled with what they
mean, and never folds them into `not-started` or merges them across repositories. A repository whose
records are mostly `open` takes part, with that fact visible. Whether taking part should instead
require a type to declare `progress` is the vocabulary question left open (open question 1).

### 6. Finding 2: the export carries no format version

Today's export has no version field. Its top-level keys are `status`, `record_count`, `types`,
`type_roles`, `read`, `records`, `roadmap` and the `next_action` trio. Repositories also pin
different releases: this repository's CI pins `v0.4.7` for its corpus gate
(`.github/workflows/ci.yml:174`), and the 2026-11-23 survey reports that the siblings run 0.4.13. `MILE-360` (Planned, not built) adds
a `paused` state that older exports never emit. A reader of several exports therefore needs to know
which vocabulary each export was written in.

The proposal: this RFC depends on `MILE-359`, which gives every report, `export` included, a
`schema_version`. The snapshot envelope copies that `schema_version` and records the
`urzua_version` that wrote it. A reader refuses an export whose `schema_version` it does not know,
with one finding, and counts its references eligible and unexamined. This RFC proposes,
as its own policy, that adding a value to the state set (such as `paused`) bumps the version.
`MILE-359` sets no bump policy, and its addition-only rule arguably implies that additions do not
bump. That conflict is for the ADR on `MILE-359` to settle. Under this proposal an older reader refuses
a newer export rather than misreading `paused`. A newer reader shows that an older export cannot
express `paused`, and does not claim the repository has nothing paused.

### 7. Finding 3: this reverses part of RFC-27, and how that is recorded

`RFC-27`'s amendment of 2026-11-08 (line 169) says: "A generated presentation of the records, such
as the dashboard (`ADR-37`), and the org rollup (`MILE-24`) stay ecosystem: they are artifacts built
around the engine's output for an audience, not a reading the engine computes inside `check` or
`roadmap`, and a function of the records alone is not enough to make one engine scope." An org view
inside `urzua board` is a generated presentation and an org rollup. Putting it in the engine reverses
that sentence for this case. Calling it a clarification would be wrong.

Proposed record of the reversal, not made here because `RFC-27` is Accepted and its text is never
rewritten:

- A new ADR, deriving from this RFC, that narrows `RFC-27`'s 2026-11-08 amendment for one class.
  It states the test below, names what moves out of `MILE-24`, and says `RFC-27` §3 otherwise
  stands.
- A dated `## Amendment` on `RFC-27`, added only once that ADR is accepted, that points at the ADR
  and changes none of `RFC-27`'s existing text.
- `MILE-24` split. The cross-repository decision graph and the org rollup move to a new milestone
  that implements the ADR. Drift trends from collected events and large-corpus performance stay in
  `MILE-24` and stay ecosystem.

The narrower test, proposed for that ADR: **a cross-repository reading is engine scope when it is
computed only from exports the engine itself wrote, keeps each record in its own repository's
vocabulary, holds no state between runs, and, for a snapshot source, is reproducible from the
committed snapshots alone.** A checkout source (section 2) gives the same reading of whatever tree is
checked out, which is reproducible only as far as that tree is; `check` in CI would read snapshots
only (open question 3).

This test also admits `ADR-37`'s generated dashboard. `scripts/generate-dashboard.py` builds its data
only from `urzua check`, `explain`, `graph` and `export --format json` through `run_urzua`
(lines 176-186), and holds no state between runs. Nothing in the code distinguishes it from a board
reading the same output. What remains is where each ships and who chooses the layout: `urzua board`
is a command of the engine binary with one fixed layout (`ADR-103`), and the dashboard is a script
in this repository with a layout curated for this project. That is a packaging difference, not a
property of the reading. Two consequences follow, and the maintainer chooses between them (open
question 7): either the ADR accepts that this class covers both, and the dashboard is engine-scope
eligible while staying a script; or the ADR adds a criterion that the reading ships in the engine
binary with no audience-specific layout, which excludes the dashboard by packaging and should say
so. The test stays true once the board reads other repositories' exports, because those exports are
the engine's own versioned output (section 6). Trend analytics over time fail the test, because they
hold state between runs, so they stay in `MILE-24`.

### 8. Finding 4: ADR-103 should be settled in the same pass

`ADR-103` (`urzua board`) is `Proposed` with `Embodiment: Not started` and does not cite `RFC-27`,
yet the command ships. Before the board takes on a cross-repository view, the maintainer would need
to decide: whether to accept `ADR-103` as written, or amend it first to cite `RFC-27` and the test in
section 7; and whether its embodiment is evidenced by the shipped command's tests, so that
`urzua fix --apply` can derive `Embodiment`. This RFC changes neither status.

### 9. Finding 5: failure modes of a cross-repository reference

- **A stale snapshot that passes.** Every finding and the info notice name the snapshot's
  `generated_at`, as `citation.issue-exists` does. Open question 5 asks whether a declared maximum
  age should make an expired snapshot unexamined.
- **A private target.** The snapshot is committed into the citing repository, so it copies the
  target's titles and statuses there. A refresh writes only what the sibling's export would show to
  whoever runs it, and a repository that must not hold a private sibling's records declares that
  sibling with no source. Its references are then eligible and unexamined, never resolved against
  data the reader cannot see. `RFC-11`'s declared boundaries apply to the snapshot file like any
  other path.
- **A renamed repository.** The qualifier is the local declared name, not the forge name. A rename
  changes one declaration and the snapshot's `repository`. The mismatch with the declared identity
  produces one finding, and references re-resolve on `stable_id` without edits.
- **Circular references.** Resolution reads one level, the snapshot of the target. It never follows
  the target's own qualified references, so a cycle across repositories cannot loop. `roadmap`
  ordering stays within one repository, and a cross-repository `Depends-on` is shown, not
  sequenced.
- **A target whose status changes after the citing record was accepted.** The citing check reports
  the target's current status from the snapshot, exactly as `pointer.resolution` does for a local
  target. `narrative-field.stale` fires on a terminal one. A change is only seen after a refresh,
  which is why `generated_at` is shown.

### 10. Finding 6: `stable_id` uniqueness across repositories

The design keys on `(repository, stable_id)`. It never needs global uniqueness to resolve a
qualified reference. A bare-`stable_id` reference does need it: the proposal resolves a bare ULID
by searching every loaded snapshot, and reports a collision as an ambiguous reference rather than
picking one. The org view reports two repositories sharing a `stable_id`, with an `info` finding.
ULIDs make that unlikely but not impossible, since records can be copied between repositories.

## Alternatives considered

- **An org-level merged config.** Rejected by the maintainer's direction ("configs never merge").
  It also contradicts `SPEC-1`'s config-driven thesis, under which one binary serves repositories of
  different shapes. A merged config would make one repository's rules depend on another's, and the
  resolution order would become a second config format (the same worry as `SPEC-3`'s open question
  on scoped overrides, line 2013).
- **A separate ecosystem tool on `urzua export` only.** It would keep `RFC-27` intact. But a
  reference from one record to another repository's record is a property of the record, and
  checking it is what `pointer.resolution` does locally. Done outside, a citing repository's `check`
  would pass references nothing examined, the silent no-op `SPEC-1` forbids. The org view alone
  could live outside; open question 8 asks the maintainer to choose.
- **Doing nothing.** `MILE-358` and `MILE-359` keep naming another repository's work in prose that
  nothing checks. Each organisation then writes its own cross-repository script, the hand-written
  linter `SPEC-1` exists to replace.

## Open questions
1. **A shared vocabulary without a merged config.** Taking part in an org view may need one carve-out
   from "configs never merge". Options: (a) none, and the view shows each repository's `open` and
   `none` as they are (section 5); (b) a versioned vocabulary file the engine publishes, such as what
   a parking status means or that `progress` is required to take part, which a repository opts into
   by name and version without importing any rule; (c) a per-sibling declaration in the reading
   repository mapping that sibling's statuses, which is rejected here as a merged config by another
   name. The maintainer decides.
2. **Where the list of siblings lives.** Options: each repository's `.urzua/config.yaml` (the
   citations a repository can make are part of its governance); a workspace file outside any one
   repository, for the org view only (which comes close to an org-level config); or both, with the
   workspace file reading only and never governing.
3. **Snapshot or checkout.** The snapshot is reproducible and offline, and suits CI. The checkout is
   current, and suits local work. Options: snapshot only; both, with `check` in CI reading only
   snapshots; checkout only (rejected here for CI).
4. **ADR-103.** Accept it as written, or amend it to cite `RFC-27` and section 7's test first
   (section 8).
5. **The level when a sibling is unread.** Options: `info` (declared, not read), or `warn`, or
   `error` past a declared maximum snapshot age.
6. **Prefix syntax.** `<repo>:<id>` collides with nothing in today's id grammar (`ADR-64`'s
   structured parsing), but `<repo>/<id>` or a URL form are alternatives. The maintainer chooses.
7. **Does the scope test admit the dashboard?** Accept that section 7's test admits `ADR-37`'s
   dashboard as well, or add a packaging criterion that excludes it, stated as packaging.
8. **Engine or ecosystem for the org view.** Qualified references belong in `check` either way. The
   org view could instead be an ecosystem tool reading the same snapshots, which leaves `RFC-27`
   intact and needs no ADR narrowing it. Options: in `urzua board` (the maintainer's relayed
   direction allows it); outside the engine; or outside first, moved in once a second organisation
   asks.

## Non-goals
- Merging, inheriting or overlaying configuration across repositories.
- Network access or a forge integration in `check` (`RFC-14`).
- Cross-repository evidence pins: a `Realized-by` locator `@sha256` pinned to a file in another
  repository. Deferred to a follow-up milestone once qualified references exist.
- Sequencing `roadmap` across repositories.
- Drift trends and large-corpus performance, which stay in `MILE-24`.
- Deciding `RFC-27`, `ADR-103` or any status. This RFC proposes.

## References
- `RFC-27` §3 and its amendment of 2026-11-08 (line 169), the part this proposes to narrow.
- `RFC-11` (line 381) and `SPEC-3` (lines 782-835, 644, 2018): the deferred non-goal, the snapshot
  model, `external_ids`, the back-pointer obstacle.
- `RFC-14` (line 128): no forge surface for a cross-repository graph.
- `MILE-24`, `MILE-358`, `MILE-359`, `MILE-360`, `ADR-103`, `ADR-37`, `SPEC-1`.
- `rust/crates/urzua-core/src/view_model.rs:25-46`, `state_of` at 218-259.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Initial RFC, `Status: Draft`. Filed from the maintainer's polyrepo direction of 2026-11-23, relayed from the organisation's coordination thread. | **structural** |
> | 2026-11-23 | Review fixes: the scope test admits the generated dashboard, stated plainly with the choice put to the maintainer; the version-bump policy stated as this RFC's own; reproducibility limited to snapshot sources; open questions 7 and 8 added; motivation bullets marked as paraphrase; SPEC-3 line corrected; sibling-repository facts labelled as survey reports. | **structural** |
> | 2026-11-23 | Corrected the survey's count for the third sibling repository: 7 `open` records, 3 rfc and 4 spec, not most or all of its records. The point it supports is unchanged. | **structural** |
