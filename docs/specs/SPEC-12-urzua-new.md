---
Stable-Id: 01M6R3NK5C0WX34AQKF79DQDS5
Version: '0.20'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:1f7d3b1732152bd0af6a555d3f6d5f128115ce34e23d4e8943af5b455cc1d994, code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs@sha256:c319a5750aa11e1bd2b6be9fc6a2d707f34150c514d49c35290ad353f0ef61f7
Author: beauwilliams
Subject: '`urzua new` -- generating a record''s initial content with a real stable ID assigned.'
Implements: ADR-27
Parent: SPEC-1
---
# SPEC-12 — `urzua new`

## Purpose

Creates a record from the type's configuration with a stable ID assigned. Never asks the author to
pick a number. Fills in only what the tool can compute; every decision a human makes stays the
template's own placeholder text for a human to pick.

## How a record is built

The header is always synthesized as YAML frontmatter from the type's `required_fields`, the only
header shape `new` writes (the `blockquote` and `bold-list` shapes were removed, `MILE-85`): every
configured field is emitted as a blank frontmatter key, so the config alone drives what an author
has to fill in. Fields `new` can compute are filled: a freshly generated `Stable-Id`, the date and
author fields (`relation_fields.date`/`author`, default `Date`/`Author`; the author resolved the same way `fix --apply` resolves an identity: `gh api user` wins even over an
explicit `--by`, which wins over `git config user.name` -- SPEC-8's identity tiering, reused rather
than redesigned), and the type's status field, which starts at the first value of the type's
declared `statuses.values` (`MILE-151`). That is the vocabulary's entry state rather than a
decision, so moving a record on from it stays a human's; a type that declares no `statuses` leaves
the field blank. A type may also name the field that takes the record's title
(`relation_fields.title`, a spec's `Subject`) and the version a record starts at
(`revision_log.version_bump.initial`, written into the `version_bump` field, a spec's `Version`), so a
scaffold does not fail `field.quality` on a required field the tool could fill (`MILE-74`). Every
other field is a decision a human makes and stays blank.

If a checked-in template exists (`.urzua/templates/<type>.md`), its body scaffolding (the H1 and the
sections beneath the header) is appended under the synthesized header; the template's own header, in
whatever shape, is never used as the header, but the field names it lists are read (see below) (`BUG-3`: the configured `header_shape` wins unconditionally over a
leftover template).

A type declaring `header_shape: none` has nowhere to write a `Stable-Id`, so `new` refuses it
(`ADR-50`).

## The declared identity pattern and field set (`MILE-151`)

`new` reads the same declarations `check` does, so a generated record passes the type's own rules
without an edit:

- **Identity.** A type that declares `identity` (`SPEC-3`) is numbered by that pattern, not by the
  `TYPE-NNNN-slug.md` convention: the next number is one past the highest `number` the pattern reads
  from a file in the directory. The file name is `PREFIX-NUMBER-slug.md` with the number zero-padded
  to the narrowest width the pattern accepts (`\d+` takes none, `\d{4}` takes four), and the H1 opens
  `PREFIX-N` with the number padded to that same width (`ADR-0044`), derived from the file name written
  beside it so the two ids agree (`BUG-188`); the title form the declaration implies (`RFC-44`). A regular
  expression cannot be run backwards, so the name is kept only if the pattern reads it back as the
  type's prefix and the number; a pattern no such name fits is a refusal naming the pattern, before
  anything is written. A type with no `identity` is numbered and named exactly as before.
- **Template fields.** A field the template's header names beyond the type's `required_fields` is written
  blank when the type declares it (`known_fields`, or any field when none is declared), so the record starts
  with the fields the template meant it to carry (`BUG-188`). A template field the type does not declare is
  left out, since it would only trip `header.field-set-consistency`.
- **Field set.** `Stable-Id` is always written (ADR-21: every record type gets one). When the type
  declares a closed field set (`known_fields`) that does not list it, the declaration contradicts
  ADR-21 and `header.field-set-consistency` will report the new record; `new` does not hide that by
  omitting the identity. It writes the `Stable-Id` anyway and adds a warning notice naming the
  inconsistency and the fix (add `Stable-Id` to the type's `known_fields`).
  A type that lists `Stable-Id` under `required_fields` gets the generated value there, once; it is
  never written as a blank required field.

## Display-number assignment

A type whose directory does not exist yet is numbered from 1 and the directory is created when the record is written; `new` says so on stderr.

Scans the type's directory for the highest existing numeric prefix and takes the next integer —
**never reused**, even if a number was deleted, since a cross-reference elsewhere may still assume
the old numbering held (the same reasoning RFC-13 argues for vacated numbers generally, applied here
as prevention: `new` never manufactures the collision RFC-13 exists to describe after the fact).

**Other refs are consulted too (MILE-89).** After the working-tree scan, `new` also lists the type's
directory on every local branch and remote-tracking ref (`refs/heads/*`, `refs/remotes/*`) and takes
one past the highest number found anywhere. Each number above the working tree's own highest that
another ref already uses is reported in the `skipped` list of the report (below), never on stderr. This is git only, with
no network and no `gh`: **a ref that was never fetched into this clone is invisible**, so a branch
or PR known only to the remote can still collide -- run `git fetch` first, and still check open PRs
before opening one. If git cannot be read (git missing, `.git` unusable), `new` numbers from the
working tree exactly as before and says so in a warning notice (subject `new.numbering`); it never silently skips the check. The query is
one `for-each-ref` plus one `ls-tree` per distinct commit: it reads a single directory tree per tip
and knows which ref each name came from, where `git log --all --name-only` walks all history and
cannot attribute.

**Open pull requests are consulted too, opt in (`ADR-93`, `MILE-89`).** With `new.check_open_prs: true`
(`SPEC-3`) `new` goes further than local refs. Absent or false, none of this runs: `new` behaves as above and
touches no remote.

- **The fetch.** `git fetch --no-tags --quiet <remote> '+<pattern>:refs/urzua/pr/<segment>/*'` for each entry
  of `new.pull_ref_patterns` (default `refs/pull/*/head`, GitHub's layout; another host names its own, such as
  GitLab's `refs/merge-requests/*/head`), from `new.remote` (default `origin`). Plain git: no `gh`, no
  GraphQL, and git's own credential helper does any authentication (a prompt is never shown). Each pattern has
  its own namespace so two patterns that both yield `123` cannot overwrite each other: `<segment>` is the
  remote's name when one pattern is configured and the pattern's zero-based index when several are. Every
  reader (numbering, `identity.open-pr-collision`) reads all of `refs/urzua/pr/` recursively, so a segment left
  by an earlier layout only reserves more numbers, never fewer. A hung fetch is killed after
  `new.fetch_timeout_seconds` (default 30). `--filter=blob:none` is not used: it turns a clone into a partial
  clone (it sets the remote's `promisor` configuration), which a numbering command must not do, and only trees
  are read either way.
- **What it sees.** Every pull request the host publishes a head ref for, forks included, **open, merged and
  closed alike**: plain git sees no state. A merged pull request's number is already on `main`. A closed,
  unmerged one keeps its number reserved, which leaves a gap in a sequence and never a duplicate; the cost is
  bounded by a project's abandoned pull requests and is the price of not calling an API. Numbering skips every
  number found under `refs/urzua/pr` or on any branch, as above, and names the refs that held it.
- **A lock serializes runs.** `<git-common-dir>/urzua-new.lock` (the git common directory, so linked worktrees
  share it) is created with `create_new`, holds the owner's pid, and is held from numbering through the file
  write; the type's directories are read under it. A second run waits, retrying for `new.lock_timeout_seconds`
  (default 10), then exits 2 naming the lock's path and holder pid. A lock whose pid is gone is removed. The
  fetch runs before the lock is taken, so the network is not held under it.
- **Every worktree is scanned.** While holding the lock `new` also lists the type's declared directories in each
  tree `git worktree list --porcelain` names, untracked files included, and skips any number found there, so a
  record just created and not yet committed in a sibling worktree is seen (a directory listing per worktree, no
  git object reads). A listed worktree whose directory is missing is skipped with a notice. **Separate
  clones remain a limit**: two clones share neither the lock nor a worktree list, so an untracked record in
  another clone is undetectable, and the fetch cannot see it either; an info notice says so.
- **Offline is never a failure.** If the remote is missing, the fetch fails or it times out, `new` numbers from
  the local refs it has, including `refs/urzua/pr/*` left by an earlier fetch, and adds a warning notice
  (subject `new.pull-refs`) naming the reason and that a number claimed on an unfetched pull request will not
  be seen. Exit is `0`.
- **The report says what was used.** `pull_refs` is always present: `fetched` (bool), `reason` (`null` when
  fetched; `disabled`, `offline` or `timeout`), `remote` (`null` when disabled), `refs` (the count under
  `refs/urzua/pr`, fetched now or earlier), `worktrees_scanned`, and `skipped_numbers`, a list of
  `{number, held_by}` naming the refs or worktrees that held each number skipped.
- **The skipped numbers are in the report, not on stderr (`ADR-93`, Amendment of 2026-11-07).** `new`'s JSON has
  a top-level `skipped` list, always present and empty when nothing was skipped, with one `{number, reason}`
  entry for each number and reason: `open-pr <ref>` when a ref under `refs/urzua/pr` holds the number (the full
  ref name), `worktree <path>` when a sibling worktree does, and `local` when a local branch or remote-tracking
  ref does (one entry however many such refs hold it; `pull_refs.skipped_numbers[].held_by` names them). It is
  ordered by number, then open pull requests, worktrees and `local`, then by text. When the pull-request check
  is off, `pull_refs.reason` is `disabled` and only local refs were consulted. Nothing about a skipped number
  is written to stderr.
- **Never reused still holds.** This only widens the set of taken numbers, with pull refs and sibling
  worktrees; it never frees one (`ADR-27`).

Filenames emitted going forward carry the type's prefix (`ADR-38-slug.md`, ADR-36); legacy
`NNNN-slug.md` filenames are never renamed and resolve identically forever.

A title that yields an empty filename slug (empty, whitespace, or only non-ASCII characters) is an
error, reported before anything is written and without consuming a number: a slug-less filename is not
one the numbering scan recognises, so the number would be reissued (`BUG-167`).

## Output

Stdout is the JSON report (ADR-23/46): `path`, `display_number`, `stable_id` (always populated), `pull_refs` (always present, see above), plus an optional
`notices` array (ADR-46) carrying non-fatal observations -- e.g. an explicit `--by` diverging from
the identity `resolve_identity` actually used -- omitted entirely when empty. `new` still does not
echo the generated content back; the caller reads the file at `path` if it needs to.

## Purity boundary

`urzua-core::new_record` is the pure render layer (`render_from_template`, `render_synthetic_yaml`),
following the same pure-render/impure-caller split as `fix.rs`: the CLI resolves identity, reads the
template file, scans the directory for the next number, and writes; `new_record`'s own functions
take content and parameters, returning a string.

## What's deliberately not built

- **Any header shape other than YAML frontmatter.** The `blockquote` and `bold-list` shapes were
  removed (`MILE-85`), so there is no other shape for `new` to write.
- **Interactive prompting** for `Status`/`Deciders`/etc. — `new` is a one-shot, scriptable command;
  a human or agent edits the placeholder text afterward in whatever tool they're already using.

## References

- ADR-27 — the decision this spec details: what `new` fills in versus leaves as placeholder.
- ADR-3/21 — the stable-ID model `new` assigns from on creation.
- ADR-36 — type-prefixed filenames going forward, legacy names unaffected.
- SPEC-1 — `new`'s original one-line contract this spec fills in the mechanism for.
- SPEC-8 — the identity-resolution tiering `new` reuses from `fix --apply`.
- BUG-3 — the template-priority defect this spec's "configured shape wins unconditionally" rule
  fixes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec. **Why:** MILE-77 found `new` documented only as an ADR while comparable-complexity command areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | §Output's "No other fields" is now false: `notices` (ADR-46) can carry a non-fatal identity-divergence observation. **Why:** this spec's own literal wording would otherwise contradict the shipped output the moment `--by` diverges from an authenticated `gh` login. | **substantive** |
> | 2026-09-11 | Corrected §"The three paths"' identity-resolution order, stale since ADR-31's amendment (`gh api user` wins even over `--by`, which wins over `git config user.name` -- not "`--by` wins outright" as this spec still said). **Why:** caught by review before merge; the same stale order SPEC-8 was already corrected for elsewhere in this same change. | **substantive** |
> | 2026-11-02 | §Display-number assignment now also skips numbers used on other local branches and remote-tracking refs, naming them on stderr, and documents that unfetched refs are invisible (`MILE-89`). Bumped to `0.5`. **Why:** `urzua new` re-assigned numbers already claimed by open PR branches. | **substantive** |
> | 2026-11-02 | Bumped to `0.6`. A title with no slug is refused. **Why:** `new` wrote a slug-less `ADR-N-.md` that its own numbering scan ignored, so the next `new` reissued `N` (`BUG-167`). | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. A type's declared `identity` now decides numbering, the file name's padding and the H1's `PREFIX-N` form; the status field starts at the first declared status value instead of a literal `null`; `Stable-Id` is left out (report `stable_id: null`, warning notice) when a closed `known_fields` does not list it. A pattern `new` cannot name a file for is refused. **Why:** `MILE-151` and `BUG-182` -- the generator and the checker read two grammars, so its output failed `filename.identity-pattern` and `field.quality`. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. `Stable-Id` is always written and the report's `stable_id` always populated; a closed `known_fields` that omits it yields a warning notice naming the inconsistency instead of an omitted identity. **Why:** `MILE-151` omitted it, contradicting ADR-21 (every record type gets a Stable-Id) and leaving a record with no identity. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. `new` always synthesizes a YAML frontmatter header and appends a template's body scaffolding when one exists; the "fill the template in" path for a `blockquote` or `bold-list` template is gone. **Why:** `MILE-85` removed those shapes. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. The H1 id is padded to the file name's width; the header fields a template names that the type declares are kept, blank. **Why:** `BUG-188` -- an unpadded H1 beside a padded file name, and optional template fields dropped. | **substantive** |
> | 2026-11-04 | Bumped to `0.11`. The fields filled with the date and the author are the type's `relation_fields.date` and `relation_fields.author` (default `Date`, `Author`), not literals. **Why:** `MILE-176`. | **substantive** |
> | 2026-11-05 | Bumped to `0.12`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs, code:rust/crates/urzua-core/src/new_record.rs, test:rust/crates/urzua-cli/tests/new_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.13`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.14`. `urzua new` reports a could-not-run when the highest existing number is the largest a `u32` holds, instead of overflowing and reissuing a number. **Why:** Phase 0 hardening of the core model. | **substantive** |
> | 2026-11-05 | Bumped to `0.15`. `new` creates the type's directory for its first record instead of failing with `could not read`. **Why:** Phase 0 hardening from an adopter switchover -- the first `urzua new waiver` failed. | **substantive** |
> | 2026-11-06 | Bumped to `0.16`. A type that requires `Stable-Id` gets the generated value in that field instead of a blank. **Why:** maintainer decision of 2026-11-05 made `Stable-Id` required on every record type in this repository, and the blank required field overwrote the generated identity. | **substantive** |
> | 2026-11-07 | Bumped to `0.17`. §Display-number assignment documents the opt-in open-pull-request check (`new.check_open_prs`): the fetch of pull-request head refs into `refs/urzua/pr/<segment>/*` with plain git, one namespace per pattern; numbering that skips the numbers held there, including closed and merged pull requests, and in sibling worktrees (untracked files included); the `<git-common-dir>/urzua-new.lock` lock; the offline fallback with a notice and exit 0; and the report's always-present `pull_refs`. Separate clones are stated as the remaining limit. **Why:** `MILE-89` / `ADR-93`. | **substantive** |
> | 2026-11-07 | Bumped to `0.18`. The numbers `new` skipped are reported in a top-level `skipped` list in the JSON, each with its reason (`open-pr <ref>`, `worktree <path>` or `local`), and no longer on stderr; the working-tree-only fallback is a warning notice (`new.numbering`). **Why:** Maintainer decision, 2026-11-07, recorded as an amendment to `ADR-93`: the stderr lines contradicted `ADR-46` decision 8. | **substantive** |
> | 2026-11-07 | Bumped to `0.19`. `new` fills the field a type names under `relation_fields.title` with the record's title and the `revision_log.version_bump` field with `version_bump.initial`; a type declaring neither starts them blank as before. **Why:** `MILE-74` -- `urzua new spec` wrote a blank `Version` and `Subject`, so the record failed `check` before anyone edited it. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/new.rs@sha256:5b61c497ff7e, rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5f, rust/crates/urzua-cli/tests/new_integration.rs@sha256:c319a5750aa1 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.20`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/new.rs` 5b61c497ff7e -> e4ca70265a45. Reason: Read the diff of each re-pinned file against main. cli/src/commands/new.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/new.rs` e4ca70265a45 -> 1f7d3b173215. Reason: Read the diff of each re-pinned file against main. cli/src/commands/new.rs: Notice struct literals replaced by Notice::new (a constructor with detail: None), keeping the function inside the SPEC-22 ceiling; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
