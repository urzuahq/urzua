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
their configurations. Each repository keeps its own `.urzua/config.yaml` and rules. The engine takes
the data side: a versioned `urzua export`, committed snapshots of sibling repositories' exports, and
qualified references (`<repo>:<id>`) to a sibling's records, which `check` resolves offline against
those snapshots and counts eligible and unexamined when a snapshot is missing or unusable. The
cross-repository org view is built outside the engine, in the board as the first ecosystem client,
reading the same snapshots. `RFC-27` stands as accepted: citing a sibling's record makes a record
more verifiable, which is engine scope by its §2 test, and the org view is ecosystem by its §3. The
maintainer's answers of 2026-11-23 are folded in below; no question stays open.

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

The maintainer then changed direction on where the view lives, quoted as relayed: "maybe somehow we
should pull the board out of urzua? Like basically the 'client' for the ecosystem?" The answers
to this RFC's open questions are listed under "Answered" below: questions 1 to 8 as relayed and
paraphrased, questions 9 and 10 quoted from the maintainer's own answers.

This repository's own records show the gap:

- `MILE-358` (line 19) has to say "a gate tool", and `MILE-359` (line 38) "the published gate-report
  JSON Schema", for work defined in another repository. Neither record can cite that work, so nothing
  checks that it exists, what state it is in, or whether it changed after these milestones were
  filed.
- `RFC-11` lists "Cross-repo reference resolution. Roadmap Phase 4" as a non-goal (line 381), and
  `SPEC-3` defers it as well. Its open question on back-pointer patterns (line 2018) says fixing them
  "is what makes cross-repo resolution possible". No record has taken this up since.
- `MILE-24` (Planned, Phase 4) holds "a cross-repo decision graph" and "an org rollup". `RFC-27`
  §3 and its 2026-11-08 amendment class both as ecosystem, and this RFC leaves them there.
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

### 2. Sibling snapshots, and a workspace file that is local tooling only

The maintainer chose committed snapshots for CI and a workspace file for the sibling list (Q2,
Q3). The file layout, the envelope and the workspace file's form below are the author's proposal.

`check` in CI reads siblings only from snapshots committed in the citing repository, one file per
sibling at `.urzua/siblings/<name>.json`. Each snapshot is self-describing: `{name, repository,
commit, generated_at, schema_version, urzua_version, records}`. `name` is the qualifier used in
references and must equal the filename; `check` reports a file whose envelope `name` differs from
its filename. `repository` is the sibling's forge identity (`owner/name`), `commit` is the sibling
commit the export was taken from, and each record carries a content hash. This follows the issue
snapshot `citation.issue-exists` already uses (`SPEC-3`, "Cited issues and pull requests", lines
782-835): written by a refresh command, committed, checked for `schema_version`, with `generated_at`
on every finding. CI never needs the network, a second checkout or any file outside the
repository.

**The committed snapshot files are the repository's declaration of its siblings** (Q10: "Snapshot
files only (Recommended)"; nothing in config names them). A sibling a repository may
cite is one it has a snapshot file for. A `<repo>:<id>` whose `<repo>` has no snapshot file is
reported, at `warn`, as "not declared". A sibling whose records must not be copied in (section 9) is
declared by committing a stub snapshot: the envelope with no records.

A **workspace file** (author's proposal for the form the maintainer's Q2 answer takes) maps sibling
names to local checkout paths and holds nothing else: no rule, no level, no vocabulary. It is local
tooling. The snapshot refresh command reads it to find each sibling's checkout, and the client's org
view reads it to find the repositories to show. A person keeps it wherever suits them, in a home
repository or outside any repository, and it is never committed into each repository. `check` in CI
never reads it.

Locally, following the Q3 answer (a local checkout is used when present), a `check` run outside CI
resolves a sibling against its checkout when the workspace file names one that is present, and
against the committed snapshot otherwise. The report names the source it read for each sibling, so
a local result that differs from CI's is visible as a different source, not a silent disagreement.

This is how a workspace file squares with "configs never merge": it governs nothing. Each
repository's own config declares the maximum snapshot age and the levels its references are held to
(section 9).

Nothing reaches a forge or a service. `RFC-14` already judged (line 128) that "a cross-repo graph has
no natural forge surface". A reference to a sibling with no snapshot is reported as "not declared",
and one whose snapshot is a stub is counted eligible and unexamined. Neither is skipped silently.

### 3. Qualified references

A reference to another repository's record is written `<repo>:<id>` (for example `<repo>:RFC-2`), the
syntax the maintainer chose, or as a bare `stable_id`. Display ids collide across repositories: several of the four repositories each
have an `RFC-1` (as reported by the 2026-11-23 survey). An unqualified display id therefore always means a record in this repository,
as it does today. In the snapshot, a qualified reference resolves to the target's `stable_id`, and
that `stable_id` is the key the engine stores. The display id is what a person reads. Once resolved:

- `pointer.resolution` reports the target's existence and status, as it does for a local target.
- `narrative-field.stale` fires once the target reaches a terminal state in its own repository's
  vocabulary (`finished` or `dropped`, section 5).
- A file in a sibling is cited `<repo>:path` and checked by `citation.path-exists` against the
  snapshot's list of paths. A first reader reports it does not need this for
  schemas: it vendors a sibling's schemas at a tag with checksums and a version file, and cites the
  local copy with an ordinary pinned `code:` locator, which already works and is stronger than a
  citation. This form is therefore the lowest-priority part of the proposal and may be deferred. A
  `<repo>:path` citation is not a stand-in for vendoring: it proves a path existed when the snapshot
  was taken, not what the file contains or at which tag. A citation follows the sibling as it moves;
  a vendored copy holds exact bytes until someone commits a new one, and a consumer may use both.
- A reference resolves to a record, not to a section in it. A first reader cites sections in prose
  ("RFC-27 §2") and reports that record-level resolution is enough.
- A qualified reference may carry a pin, `<repo>:<id>@sha256:<hash>`, taken from the record's
  content hash in the snapshot. Writing it extends `urzua fix --pin`, which today pins only `code:`
  and `test:` locators in `Realized-by` (`SPEC-8`). After a refresh, a pinned reference whose
  target's hash changed is reported the way a stale evidence pin is today, with a message such as
  "cited record changed since it was pinned" (proposed wording), and is re-pinned with
  `urzua fix --rebaseline` and a reason. An unpinned reference reads only the
  target's current status. Whether a reference must be pinned is a policy each repository declares
  in its own config (Q9, below), not a rule the engine fixes.

A reference is unexamined, and counted as eligible and unexamined, when the sibling has a stub snapshot,
its snapshot is unparsable, of another `schema_version`, or named differently from its file, or the
target is not in the snapshot. In each case one finding names the reason. The report keeps the
existing split of examined against found (`SPEC-1`: no silent no-op), so a check whose snapshot
never loaded cannot pass as one that found nothing.

`external_ids` (`SPEC-3`, line 644) stays as it is. It names ids that are deliberately not records
anywhere urzua reads (an IETF RFC number, for example), which neither resolve nor report. A qualified
reference is the opposite case: an id that is a record and should resolve. A repository moving an
id from `external_ids` to a qualified reference gains a check and loses nothing.

### 4. The org view is an ecosystem client

The org view is built outside the engine, in the board as the first ecosystem client (the
maintainer's change of direction, section 7). It reads the committed snapshots, or the exports of
the checkouts the workspace file names. It labels every record with its repository, keys it on
`(repository, stable_id)`, and shows the qualified references between records as edges. It shows
`state` as each export gives it, grouped by repository and then by state, and never recomputes
`state` from raw statuses with a merged vocabulary. Nothing in `check`, `fix` or the record schema
depends on it (`RFC-27` §3).

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

The proposal: the org view shows `open` and `none` as their own columns, labelled with what they
mean, and never folds them into `not-started` or merges them across repositories. A repository whose
records are mostly `open` takes part, with that fact visible.

A repository can do better by opting in to a shared vocabulary (the maintainer's answer to the
vocabulary question). The engine publishes a versioned vocabulary file, for example what a parking
status means and which statuses are `progress`. A repository adopts it by name and version in its
own config. No rule is imported, and the adopting repository's statuses remain its own declaration.
The org view can then say which repositories share a vocabulary and at which version. A repository
that adopts none still takes part, with its `open` and `none` shown as they are.

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

### 7. Finding 3: RFC-27 stands as accepted

An earlier draft put the org view inside `urzua board` and proposed an ADR narrowing `RFC-27`'s
amendment of 2026-11-08 (line 169), which keeps "a generated presentation of the records" and "the
org rollup (`MILE-24`)" in the ecosystem. The maintainer's change of direction drops that: the board
moves out as the ecosystem's client. So `RFC-27` is not amended or narrowed, no ADR narrows it, and
no scope test is proposed. The split, by `RFC-27`'s own tests:

- **Engine scope:** the versioned export, sibling snapshots, and cross-repository citations in
  `check`. Resolving a citation against a snapshot is relationship resolution, which `RFC-27` §2
  names as engine scope; it makes the citing record more verifiable. `MILE-24`'s graph spanning
  repositories would read these citations as its edges, but resolving them is not the graph, so
  nothing moves out of `MILE-24`, which stays ecosystem as a whole.
- **The snapshot refresh:** engine scope by §2, not by the 2026-11-08 amendment. The amendment's
  class is readings computed "from the records and the configuration alone, with no state of its
  own, no service and no model call"; a refresh is not a reading of that kind but the writer of an
  input, so it is not in that class. It passes §2's test the way `urzua refs refresh` does for `citation.issue-exists`: it
  produces the committed input that makes citations verifiable. It also stays clear of what the
  amendment keeps out: it holds no state about the work, reads only records (through each sibling's
  own `urzua export`) plus the workspace file, which like the configuration is local input and no
  state about the work, and needs no service, since it reads local checkouts. In that it differs
  from `urzua refs refresh`, which needs the forge's API.
- **Ecosystem:** the cross-repository org view and the org rollup, built in the client from the
  engine's output (§3).

### 8. Finding 4: ADR-103

`ADR-103` (`urzua board`) is reworked separately (it stays `Proposed`) to treat the board as the
first ecosystem client, a Rust binary in this repository that reads only published JSON, with the
command removed when the client ships and no alias (the maintainer: "Break it, we are only users").
The export gains its format version and the config-declared data before the client is built
("Export first, then client (Recommended)"). This RFC defers to it and changes no status.

### 9. Finding 5: failure modes of a cross-repository reference

- **A stale snapshot that passes.** Every finding and the info notice name the snapshot's
  `generated_at`, as `citation.issue-exists` does. An unread or stale sibling is a `warn`, and an
  `error` once its snapshot is older than a maximum age declared in the citing repository's own
  config (the maintainer's answer).
- **A private target.** The snapshot is committed into the citing repository, so it copies the
  target's titles and statuses there. A refresh writes only what the sibling's export would show to
  whoever runs it, and a repository that must not hold a private sibling's records declares that
  sibling with a stub snapshot, envelope only (section 2). Its references are then eligible and unexamined, never resolved against
  data the reader cannot see. `RFC-11`'s declared boundaries apply to the snapshot file like any
  other path.
- **A renamed repository.** The qualifier is the snapshot's `name`, not the forge name, so a rename
  changes no reference. It changes the snapshot's `repository` at the next refresh, and
  `check --base` reports a snapshot whose `repository` differs from the merge base's copy, as one
  finding (author's proposal). References keep resolving on `stable_id`.
- **Circular references.** Resolution reads one level, the snapshot of the target. It never follows
  the target's own qualified references, so a cycle across repositories cannot loop. `roadmap`
  ordering stays within one repository, and a cross-repository `Depends-on` is shown, not
  sequenced.
- **A target whose status changes after the citing record was accepted.** The citing check reports
  the target's current status from the snapshot, exactly as `pointer.resolution` does for a local
  target. `narrative-field.stale` fires on a terminal one. A change is only seen after a refresh,
  which is why `generated_at` is shown. A change to the target's content, not only its status, is
  seen only through a pinned reference (section 3): a first reader asks for exactly this, because a
  record can build on a sibling's Draft that keeps changing, and without a pin it changes under the
  citation silently.

### 9a. Pinning policy is declared in each repository's config

The maintainer's answer to Q9 is "Maybe as a principle it's better to default to decisions being
configurable". So the engine does not fix whether a qualified reference from a decided record must
be pinned. A rule option in the citing repository's own `.urzua/config.yaml` declares it, with
values such as `none` (pins optional everywhere), `from-decided` (a reference from a record whose
status is in its type's declared `success` list to a sibling record that is not terminal must be
pinned, so a decision cannot rest on a sibling Draft that changes silently) and `always` (every
qualified reference is pinned). The option's name and its default are the author's to propose in the
ADR that decides the rule; this RFC names the shape, not the default. It is per repository, so it
leaves "configs never merge" whole. It also follows the thesis in `SPEC-1`: the same binary serves
repositories that hold their references to different standards.

### 10. Finding 6: `stable_id` uniqueness across repositories

The design keys on `(repository, stable_id)`. It never needs global uniqueness to resolve a
qualified reference. A bare-`stable_id` reference does need it: the proposal resolves a bare ULID
by searching every loaded snapshot, and reports a collision as an ambiguous reference rather than
picking one. The org view, in the client, shows two repositories sharing a `stable_id`.
ULIDs make that unlikely but not impossible, since records can be copied between repositories.

## Alternatives considered

- **An org-level merged config.** Rejected by the maintainer's direction ("configs never merge").
  It also contradicts `SPEC-1`'s config-driven thesis, under which one binary serves repositories of
  different shapes. A merged config would make one repository's rules depend on another's, and the
  resolution order would become a second config format (the same worry as `SPEC-3`'s open question
  on scoped overrides, line 2013).
- **Everything in an ecosystem tool on `urzua export`, citations included.** A reference from one
  record to another repository's record is a property of the record, and checking it is what
  `pointer.resolution` does locally. Done outside, a citing repository's `check` would pass
  references nothing examined, the silent no-op `SPEC-1` forbids. The org view does live outside
  (section 4); the citations do not.
- **The org view inside `urzua board`.** The earlier draft's proposal. It reversed part of
  `RFC-27`'s 2026-11-08 amendment and needed a scope test that also admitted the generated dashboard
  (`ADR-37`). Dropped by the maintainer's change of direction.
- **The sibling list in each repository's config.** Declined in favour of a workspace file that is
  local tooling only, with the governing parts (maximum age, levels) in each repository's config.
  Q10 confirmed it: the committed snapshot files alone are the declaration.
- **Doing nothing.** `MILE-358` and `MILE-359` keep naming another repository's work in prose that
  nothing checks. Each organisation then writes its own cross-repository script, the hand-written
  linter `SPEC-1` exists to replace.

## Open questions
None open. Questions 9 and 10 were answered on 2026-11-23 (below).

### Answered

The maintainer answered the rest on 2026-11-23 in the organisation's coordination thread. The
answers are relayed and paraphrased here, apart from the quotation in Motivation, and folded into
the Proposal.

1. **Shared vocabulary:** yes, opt-in. The engine publishes a versioned vocabulary file, and a
   repository adopts it by name and version; no rules are imported (section 5). A first reader had been
   copying this repository's `rfc` status block into its own config, and the coordination thread
   reports two sibling repositories have since copied it by hand; this answers both.
2. **Where the sibling list lives:** a workspace file, chosen over each repository's config. The
   form this RFC gives it is the author's: local tooling only, never read by `check` in CI, never
   committed into each repository, and carrying no rule (section 2).
3. **Snapshot or checkout:** both. `check` in CI reads only committed snapshots; a local checkout is
   used when present, locally (section 2).
4. **ADR-103:** being reworked separately, with the board as client and the command deprecated
   (section 8).
5. **Level for an unread sibling:** `warn`, and `error` past a maximum snapshot age declared in the
   repository's own config (section 9).
6. **Prefix syntax:** `<repo>:<id>` (section 3).
7. **Does the scope test admit the dashboard:** moot. `RFC-27` is untouched and no scope test is
   proposed (section 7).
8. **Engine or ecosystem for the org view:** outside the engine, in the client (section 4).
9. **Must a decided record's citation of a non-terminal sibling record be pinned?** The maintainer
   answered: "Maybe as a principle it's better to default to decisions being configurable". It is
   declared in each repository's own config, as a rule option with values such as `none`,
   `from-decided` and `always`, not fixed by the engine; the author proposes the default in the
   deciding ADR (section 9a).
10. **Is the committed snapshot set enough as each repository's declaration of its siblings?** The
    maintainer answered: "Snapshot files only (Recommended)". The committed
    `.urzua/siblings/<name>.json` files are the declaration and nothing in config names siblings
    (section 2).

## Non-goals
- Merging, inheriting or overlaying configuration across repositories.
- Network access or a forge integration in `check` (`RFC-14`).
- Cross-repository evidence pins: a `Realized-by` locator `@sha256` pinned to a file in another
  repository. Deferred to a follow-up milestone once qualified references exist.
- Sequencing `roadmap` across repositories.
- Drift trends and large-corpus performance, which stay in `MILE-24`.
- Amending or narrowing `RFC-27`.
- The org view and org rollup in the engine: they are the client's.
- Deciding `ADR-103` or any status. This RFC proposes.
- Section anchors in a cross-repository reference: a reference names a record.
- Gate receipts. A `gate:` locator a first reader plans (no record here plans one) cites a receipt, which is not a record and is not
  committed (ignored locally, a build artifact in CI); the committed-snapshot model does not reach
  it, and nothing here resolves one.

## References
- `RFC-27` §2, §3 and its amendment of 2026-11-08 (line 169), which this RFC leaves as accepted.
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
> | 2026-11-23 | First-reader input: the snapshot records the sibling `commit` and a per-record content hash; a qualified reference may be pinned and is reported when its target changed (open question 9); file citation marked lowest priority, since a vendored, pinned local copy already serves schemas; non-goals add section anchors and gate receipts. | **structural** |
> | 2026-11-23 | A second reader: a `<repo>:path` citation is stated not to stand in for vendoring, and open question 1 notes two siblings copied this repository's `rfc` status block by hand. | **structural** |
> | 2026-11-23 | Folded in the maintainer's answers of 2026-11-23, relayed from the coordination thread: the board moves out as the ecosystem client, so the reversal of `RFC-27`, the ADR narrowing it, its amendment pointer and the scope test are dropped; the org view is ecosystem and cross-repository citations stay engine scope; sibling snapshots committed per repository with a local-only workspace file; opt-in versioned vocabulary; `warn`, then `error` past a declared maximum age; `<repo>:<id>`; `ADR-103` deferred to its own rework. Open questions 1-8 moved to Answered; 9 stays open. | **structural** |
> | 2026-11-23 | Review fixes, labelled as the author's proposal: the snapshot envelope gains `name`, checked against its filename; the committed snapshot set is the sibling declaration, an undeclared qualifier is a `warn`, a private sibling is a stub snapshot, a rename is caught by `check --base`; open question 10 added. A local `check` reads a present checkout, as the Q3 answer says. Refresh justified under `RFC-27` §2; `MILE-24` stays ecosystem as a whole. Two typos fixed. | **structural** |
> | 2026-11-23 | Review wording: the amendment's class quoted from its definition, a paraphrased answer no longer in quotation marks, the refresh argument names the workspace file and the service `refs refresh` needs, and the `MILE-24` sentence states the edge relation instead of "never its work". | **structural** |
> | 2026-11-23 | Folded in the maintainer's answers to the last two questions, quoted: Q9, "Maybe as a principle it's better to default to decisions being configurable", so the pinning policy is a per-repository config option (new section 9a) and not fixed by the engine; Q10, "Snapshot files only (Recommended)", so the committed snapshot set is the whole declaration. Questions 9 and 10 move to Answered, none stays open, and section 8 records the board client's shape and the export-first order. The deciding ADRs are not written; no status changes. | **structural** |
