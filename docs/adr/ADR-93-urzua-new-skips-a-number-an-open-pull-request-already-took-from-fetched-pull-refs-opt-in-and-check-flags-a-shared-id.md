---
Stable-Id: 01M6SK9SN0VEYEP14T6W8MQC3D
Status: Accepted
Accepted-on: 2026-11-06
Date: 2026-11-06
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 93 — urzua new skips a number an open pull request already took, from fetched pull refs, opt in, and check flags a shared id

## Context

Line references are to `origin/main` at `cdca1179`. `new.rs`, `check.rs` and `refs.rs` are under
`rust/crates/urzua-cli/src/commands`; `lib.rs` is `rust/crates/urzua-io/src/lib.rs`.

Two records filed on two branches cut from one `main` get the same number, and each branch looks valid on
its own (`MILE-89`, found live as `RFC-22`/`RFC-22`, again on 2026-09-16 with `RFC-26` and `BUG-19`; the maintainer reports on
2026-11-06 that numbers have kept colliding when work runs in parallel). The maintainer decided on 2026-11-06 to move `MILE-89` into Phase 0 and to fetch pull-request head
refs with plain git, with no `gh` and no GraphQL.

What exists today:

- **`new` numbers from the working tree, then skips numbers claimed on local refs.** `assign_display_number`
  (`new.rs:308-330`) takes the next free number among the working-tree filenames, then skips any number
  already in a local branch or remote-tracking ref. `other_ref_claims` reads only `refs/heads` and
  `refs/remotes` with one `for-each-ref`, then one `ls-tree` per distinct commit (`new.rs:358-411`). Its own
  stderr line states the limit: "local git refs only; unfetched remotes are not checked" (`new.rs:328`). A
  git failure falls back to the working tree with a message (`new.rs:314-319`).
- **A pull request's branch is not in those refs.** A default clone fetches `+refs/heads/*`. GitHub also
  publishes `refs/pull/<n>/head` for every pull request; it is not fetched. Observed on this repository:
  `git ls-remote origin 'refs/pull/*/head'` lists 378 refs, and `git fetch origin
  '+refs/pull/*/head:refs/urzua/pr/*'` brings them in, after which the existing `ls-tree` pass over their
  distinct tips took about one second.
- **Those refs are open, merged and closed pull requests alike.** Plain git sees no state. Observed: none of
  the 378 fetched tips is an ancestor of `origin/main`, because this repository squash-merges, so a merged
  pull request's head is not reachable from `main`. A head can be told from `main`'s own copy only by
  comparing what it adds with what `main` has (path and `Stable-Id`), not by ancestry. Whether a pull request
  is open exists only in the API. This is the main consequence of the maintainer's constraint, and the design
  below is shaped by it.
- **`new` has no lock.** The file is created with `create_new` (`new.rs:246-252`), which refuses an existing
  filename, but two concurrent runs pick the number before either file exists and write different slugs, so
  they do not collide on a filename and both succeed. `FixLock` (`lib.rs:487-513`) is `create_new` on
  `.urzua/cache/fix.lock`, fails loud when present and never expires; `new` needs to wait, not fail.
- **The toolchain pins Rust 1.87** (`rust/rust-toolchain.toml`, `rust/Cargo.toml:8`), so `File::lock`
  (stable in 1.89) is not available; a lock is a `create_new` file.
- **The only network call today is `refs refresh`.** It shells out to `curl` for the GitHub API
  (`refs.rs:150-175`) and is the command a person runs on purpose. No other command touches the network;
  `urzua` shells out to `git` for everything else (`ADR-6`).
- **`check` already has a duplicate-id rule for the working tree.** `identity.collision` (`RULE_IDENTITY_COLLISION`,
  `rules/catalog.rs:8`) reports two records that resolve to one identifier. It cannot see a record that is
  not in the tree.
- **A diff-only rule can be declared.** `ADR-90` lets a rule declare `scope: changed`, so it judges only the
  records a `--base` diff adds or changes.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Local refs only, and state the limit (today) | No network; nothing to build | Misses exactly the failure that recurred: the claiming branch exists only as a pull request nobody fetched |
| B. Query open pull requests with `gh` or the GitHub API | Knows open from closed | A new dependency and credential (`gh`, a token, or GraphQL), GitHub-only, and a second network path; rejected by the maintainer, and against `ADR-6` |
| C. `git ls-remote` only | One cheap call, no objects fetched | Returns commit ids, not trees, so it cannot say which record numbers a head holds; branch names do not carry them reliably |
| D. Fetch pull-request head refs into a private namespace with plain git, opt in (chosen) | Sees every pull request, forks included; only `git`; the existing `ls-tree` pass is reused; works for any host that exposes pull refs | Network on a command that was offline; sees closed and merged pull requests too (below); GitHub's ref layout is the default |
| E. Stop numbering sequentially (ids from the `Stable-Id` alone) | No collisions by construction | The display number is the citation surface used in every record and commit; replacing it is a different decision with a far larger blast radius |
| F. Fetch by default, no option | No config | A numbering command silently starts reaching a remote; offline, restricted-network and credential-less checkouts change behavior without being asked |

## Decision

*In the context of parallel branches that number records from one `main`, facing a maintainer decision that
the fix use plain git, we decided that `urzua new` fetches pull-request head refs into
`refs/urzua/pr/*` when `new.check_open_prs` is true, skips every number found there or on any fetched branch,
serializes concurrent runs with a local lock, and falls back to local refs with a notice when the fetch fails,
and that an opt-in `check` rule flags an id also used by a pull request under `--base`, to stop the collision
at the moment a number is chosen and again at review, accepting a network step, and that the refs include
closed and merged pull requests.*

1. **This is the first network step outside `refs refresh`, and is opt-in.** The fetch runs only when
   config says `new: {check_open_prs: true}`. Absent or false, `new` behaves exactly as today, including its
   stderr limit line, and touches no remote. This repository turns it on (`MILE-89`).
2. **The fetch.** `git fetch --no-tags --quiet <remote> '+<pattern>:refs/urzua/pr/<segment>/*'` for each
   entry of `new.pull_ref_patterns`, default `["refs/pull/*/head"]` (GitHub's layout; a repository on another
   host names its own, such as GitLab's `refs/merge-requests/*/head`). `new.remote` names the remote, default
   `origin`. Each pattern gets its own namespace, so two patterns that both yield `123` cannot overwrite each
   other: `<segment>` is the remote name when one pattern is configured, and the pattern's zero-based index in
   `pull_ref_patterns` when several are. Every reader below (numbering, the rule) reads all of
   `refs/urzua/pr/` recursively, so a segment left by an earlier layout only reserves more numbers, never
   fewer. The refs live under `refs/urzua/`, which no default refspec, `git branch` or `git log` shows. The
   build measures `--filter=blob:none`, since only trees are read, and uses it only if it is faster and the
   host accepts it. A bounded wait (default 30 seconds, `new.fetch_timeout_seconds`) kills a hung fetch.
   No `gh`, no GraphQL, no credential handling of urzua's own: git's credential helper does it.
3. **Numbering skips them all.** `other_ref_claims` reads `refs/urzua/pr` in addition to `refs/heads` and
   `refs/remotes` (`new.rs:358-411`). A number held on any of them is skipped and named in the report. Because
   state is unknown, a closed, unmerged pull request keeps its number reserved: a gap in a sequence, never a
   duplicate. Numbers held by merged pull requests are already on `main`. The cost is bounded by the abandoned
   pull requests of a project and is the price of not calling the API.
4. **A local lock serializes `new`, and numbering scans every worktree.** `<git-common-dir>/urzua-new.lock`,
   created with `create_new` (the git common directory, so linked worktrees share it) and held from numbering
   through the file write. A second run waits, retrying for a bounded time (default 10 seconds), and then fails
   with the lock's path and holder pid. A lock whose pid is gone is removed. The lock alone is not enough
   across worktrees: `new` numbers from its own tree and the refs, and a record just created, untracked, in a
   sibling worktree is on neither, so a second run would take the same number. While holding the lock, `new`
   therefore also reads the type's declared directories in every tree listed by `git worktree list --porcelain`,
   untracked files included, and skips any number found there. That is cheap and local (a directory listing per
   worktree, no git object reads). A listed worktree whose directory is missing is skipped with a notice. The
   limit that remains is separate clones: two clones share neither a lock nor a worktree list, so an untracked
   record in another clone is undetectable, and the fetch cannot see it either. The report states that limit.
5. **Offline is never a failure.** If the remote is missing, the fetch fails or times out, `new` numbers from
   the local refs it has, including `refs/urzua/pr/*` left by an earlier fetch, and adds a `notices` entry
   naming the reason and that a number claimed on an unfetched pull request will not be seen. Exit is `0`.
   This keeps the behavior of the existing git-failure fallback (`new.rs:314-319`).
6. **The report says what was used.** `new`'s JSON gains `pull_refs`: `fetched` (bool), `refs` (count),
   `remote`, and `skipped_numbers` with the refs or worktrees that held them, and `worktrees_scanned` (count). A run that did not fetch says why
   (`disabled`, `offline`, `timeout`).
7. **A `check` rule flags a shared id (opt-in).** `identity.open-pr-collision`, off unless configured, with
   `scope: changed` (`ADR-90`): for each record the `--base` diff adds, it reports a warning (its default
   severity; a repository may raise it to error) when a ref under
   `refs/urzua/pr` holds a file with the same type and number and a different `Stable-Id`. Without `--base` it
   reports that it judges a diff and does nothing, as the other diff rules do (`commands/check.rs:981`). A
   head whose file for that id has the record's own `Stable-Id` is the same record and is not a collision, so a
   pull request is not flagged against itself nor against its own merged copy. Because plain git cannot tell
   open from closed, a head that is closed and unmerged can still hold the number a new record legitimately
   took, and an error would fire against it for as long as the head exists. The default is therefore a
   warning, which still gates under `fail_on: warning`, and the path out is a waiver: a `docs/waiver` record
   for `identity.open-pr-collision` scoped to the record, whose `Reason` names the closed pull request and that
   it was not merged. The alternative, a renumber with `urzua migrate ids`, stays available when the other
   pull request is live.
8. **`check` never fetches.** The rule reads refs that are already there; fetching belongs to `new` and to one
   `git fetch` line in a workflow that wants the rule. If the rule is configured and `refs/urzua/pr` is empty,
   it reports a notice saying the refs are missing and how to fetch them, and its population counts zero refs,
   so a run over nothing cannot be mistaken for a clean one (`ADR-55`).
9. **Numbering follows `ADR-27`'s never-reuse rule.** `ADR-27` says a display number is "never reused even
   if a number was deleted", and its amendments of 2026-11-04 and 2026-11-05 widen the set of taken numbers to
   every claimed local or remote-tracking ref and every directory the type declares. This decision only widens
   that set again, with pull refs and sibling worktrees. It never frees a number, and a number it cannot see
   (an unfetched pull request, another clone) is the limit `ADR-27`'s amendment already states for an unfetched
   branch. `ADR-27` is therefore complied with and needs no amendment from this decision.
10. **`MILE-89` is the milestone for this decision**, moved to Phase 0 by the maintainer on 2026-11-06 and now
   implementing this record.

## Questions decided

The independent review of 2026-11-06 answered the six open questions; each is decided as follows.

1. **Closed pull requests reserve their numbers.** Yes (clause 3). The real gap today is 0: of 379 pull
   requests, 4 are closed unmerged (#19, #42, #53, #126), and every number they added is on `main`. The
   reservation costs nothing here now and is bounded by a project's abandoned pull requests.
2. **The collision rule is scoped to `scope: changed`.** Yes (clause 7).
3. **`check` never fetches.** Yes (clause 8).
4. **Configurable ref patterns.** Kept, with one namespace per pattern (clause 2).
5. **The lock's limit across clones and worktrees.** Worktrees are covered by the scan in clause 4; separate
   clones remain an undetectable limit, disclosed in the report.
6. **An `ADR-56` revision row.** That question belongs to `ADR-92`, where a row alone was found not enough and
   an amendment is required (`ADR-92` clause 9).

## Reversibility

Easy. The fetch, the refs namespace and the rule are all behind config that defaults off; deleting
`refs/urzua/` leaves a clean repository. The lock file is created and removed per run. What becomes sticky is
the report shape (`pull_refs`) and the config key names.

## Consequences

- `new` can name a number a parallel pull request holds, including a fork's, which the local-ref scan could
  never see. The recurring `RFC-22`, `RFC-26`/`BUG-19` and `ADR`/`MILE` collisions are prevented at the point
  the number is chosen, not found at review.
- A repository that opts in pays one fetch of its pull refs per `new` (about a second of local tree reads for
  378 refs here, plus the fetch). It also stores the pull refs' objects locally, which is the cost of seeing
  their trees; a partial-clone filter may cut it.
- Numbers held by abandoned pull requests are never reused, so a long-lived project accumulates gaps. A gap is
  honest: the number was claimed. `RFC-13`'s tombstone is the answer if a gap needs explaining.
- The `check` rule can flag a new record against an abandoned pull request that held its number. Because it is
  `scope: changed`, only a pull request that adds a record meets it, once, and the fix is a renumber
  (`urzua migrate ids`), not a standing finding.
- A network failure never blocks filing a record. The cost is a window in which a collision is possible; the
  notice names it.
- The lock and the worktree scan cover one clone's runs, including its linked worktrees. Two agents in separate
  clones still depend on the fetch to see each other, which is why the fetch is the primary defense and the
  lock the secondary.
- The first network call outside `refs refresh` sets a precedent: a later command that wants the network
  needs its own opt-in and its own decision.
- `SPEC-12` (numbering, the lock, the report fields), `SPEC-3` (the `new` key), `SPEC-2` and the rule table
  (`identity.open-pr-collision`) change in `MILE-89`.

## Amendment (2026-11-07)

Two changes, both decided by the maintainer on 2026-11-07, share this one amendment.

### The remedy for a shared number is a rename, not `urzua migrate ids`

Accepted by the maintainer on 2026-11-07.

Clause 7 and the Consequences name `urzua migrate ids` as the way to renumber a record that shares a
number with a pull request. It cannot: `migrate ids` only inserts a missing `Stable-Id` line and keeps the
filename's number, so running it on a colliding record changes nothing and an agent following the
machine-readable remedy loops. No command renumbers a record. The corrected statement is: the remedy is to
rename the file and its H1 to the next free number, the one `urzua new` would take, or to waive the finding
when the other pull request is abandoned (a `docs/waiver` record whose `Reason` names it, as clause 7
already says). The finding's message, the `open-pr-collision` action reason in the rule table and `SPEC-2`
say this. The maintainer's wording named `urzua new --dry-run` as the way to find the next number; `new`
has no `--dry-run`, so the text says what the next number is rather than naming a flag that does not
exist, and a dry-run option for `new` is a separate decision. What clause 7 decides is unchanged.

### The numbers `new` skipped are reported in the JSON, not on stderr

*Status of this amendment: accepted by the maintainer on 2026-11-07. `Status` stays `Accepted` and the Decision text above is unchanged.*

Clause 3 says a skipped number "is named in the report", and clause 6 puts the refs or worktrees that held it in
`pull_refs.skipped_numbers`. The build also printed `urzua: skipped N: already used on ...` to stderr for every
number it passed over (`new.rs:440` and `new.rs:445`, the second with the caveat "local git refs only; unfetched
remotes are not checked"), and printed `urzua: numbering from the working tree only; could not read other refs`
when git could not list the refs (`new.rs:391`). `ADR-46` decision 8 eliminates stderr for anything this code can
structure into JSON, and each of these is structured: a number and what held it, and a fallback that is a
notice. The maintainer decided on 2026-11-07 to report the skipped numbers in the JSON and drop the stderr lines.
Clauses 1 to 5 and 7 to 9 are unchanged, and clause 6 gains the list below; `pull_refs.skipped_numbers` stays as
clause 6 states it.

**1. A `skipped` list.** `new`'s JSON gains a top-level `skipped` list, always present and empty when nothing
was skipped. Each entry is `{number, reason}`, one for each skipped number and reason:

- `open-pr <ref>`: the number is held by a ref under `refs/urzua/pr`, named by its full ref name (clause 2).
- `worktree <path>`: the number is held in the tree of a sibling worktree (clause 4), named by its path.
- `local`: the number is held by a local branch or a remote-tracking ref. One entry however many such refs hold
  it; `pull_refs.skipped_numbers[].held_by` names them.

The list is ordered by number, then open pull requests, worktrees and `local`, then by text, so two runs over
the same state print the same list. A number held in two kinds of place has one entry for each.

**2. No stderr lines for skipped numbers.** The per-number `skipped` lines are removed, in both forms. When the
pull-request check is off, `pull_refs.reason` is `disabled`, which already states that only local refs were
consulted; the caveat that an unfetched remote is not checked is that field, not a stderr suffix.

**3. The git-failure fallback is a notice.** When git cannot list the other refs, `new` still numbers from the
working tree and still exits `0`, and says so as a warning notice with the subject `new.numbering` instead of
a stderr line. Nothing is silent: the check that could not be made is reported, as clause 5 requires of the
offline fetch.

Not changed: the stderr lines `new` prints for an `extra_dirs` entry that does not exist, for a type directory
it is about to create and for a failed pull-request fetch (the last also a notice). They are outside this
decision and stay on stderr until a change of their own moves them.

## References

- `ADR-6` -- shell out to git; the fetch is plain `git fetch`.
- `ADR-55` -- a rule must be able to fire and say it looked; the missing-refs notice and the population count.
- `ADR-64` -- structured config in, structured output out.
- `ADR-90` -- `scope: changed`, which the new rule uses.
- `ADR-27` -- `urzua new`; its never-reuse rule and its amendments of 2026-11-04 and 2026-11-05 govern numbering, and clause 9 states how this decision complies.
- `RFC-13` -- a vacated number is a record, not a gap; the aftermath half of this failure class.
- `MILE-89` -- the milestone this decision implements.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-06 | Drafted as Proposed. **Why:** Maintainer decision, 2026-11-06: move `MILE-89` to Phase 0; fetch pull-request head refs with plain git (no `gh`, no GraphQL), opt in; skip numbers taken there or on any fetched branch; add a local lock; fall back to local refs offline; add an opt-in `check` rule under `--base`. The maintainer accepts it in a later change. | **substantive** |
> | 2026-11-06 | Amended before acceptance; stays Proposed. Clause 4 scans the trees of `git worktree list` for claimed numbers, untracked files included, and discloses separate clones as the remaining limit; clause 2 gives each `pull_ref_patterns` entry its own `refs/urzua/pr/<segment>/*` namespace; new clause 9 cites `ADR-27`'s never-reuse rule and amendments and states compliance; clause 7 defaults `identity.open-pr-collision` to warning and documents the waiver for a closed unmerged head; the cite `new.rs:313-319` is corrected to `314-319`; the review's answers to the six open questions are recorded as decided, with the measured gap of 0. **Why:** independent review of 2026-11-06 found four blockers (the lock did not cover worktrees, patterns shared one namespace, `ADR-27` was uncited, an error rule could fire forever against a closed head). | **substantive** |
> | 2026-11-06 | Status changed from Proposed to Accepted. **Why:** Maintainer decision, 2026-11-06. | **substantive** |
> | 2026-11-07 | Amendment added: `new` reports the numbers it skipped in a `skipped` list in its JSON (`open-pr <ref>`, `worktree <path>` or `local` for each), the `skipped N` stderr lines are dropped, and the git-failure fallback becomes a `new.numbering` notice. `Status` stays `Accepted`. **Why:** Maintainer decision, 2026-11-07: the stderr lines contradicted `ADR-46` decision 8. | **substantive** |
> | 2026-11-07 | Added an `## Amendment (2026-11-07)`, accepted by the maintainer on 2026-11-07: the remedy for `identity.open-pr-collision` is to rename the file and its H1 to the next free number, or waive the finding when the other pull request is abandoned, because `urzua migrate ids` cannot renumber. `Status` stays Accepted. **Why:** Maintainer decision, 2026-11-07, on the CLI review. | **substantive** |
