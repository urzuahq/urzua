---
Stable-Id: 01M6R3NK5Q0XVK27P5T8FN72BN
Version: '0.36'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/fix.rs@sha256:5a67695c913d0e3c7127466307c786315fc9838015bfe77bbbe45b27a0819e11, code:rust/crates/urzua-core/src/fix.rs@sha256:88c815c0639acb367ed1471e6a249d646048741862208ac678c4a1e372a61518, test:rust/crates/urzua-cli/tests/fix_paths_integration.rs@sha256:73cb06a1782f02abeaf6c0795632ed1d1939633d72f67033247120656c6927d2, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs@sha256:27c91b75ad614811aa87db203bafedcadf05c64de434957d0859e2af55659bf0, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs@sha256:ff69343ca324400bde79c266372f9a213728d4d4ef8202beca6e12e54aa35352
Author: beauwilliams
Subject: '`urzua fix` -- detecting and applying Embodiment corrections computed from evidence.'
Implements: ADR-15, ADR-18, ADR-19, ADR-20
Parent: SPEC-1
---
# SPEC-8 — `urzua fix`

## Purpose

`fix` is the tool-authored repair path: it detects fields whose stated value disagrees with what the
tool can compute from evidence already in the tree, and — only under a strict eligibility test — can
write the computed value back. This spec is the complete, current build of that path: detect mode,
apply mode, the eligibility test that gates what may ever be written, and the safety mechanics apply
mode enforces as hard gates, not niceties.

## Eligibility: what a tool may ever write (ADR-15)

A field is **tool-writable** if and only if it is:

- **Derivable** — computed from evidence already in the record tree, no inference.
- **Single-valued** — one correct answer, not a judgment call.
- **Non-destructive on write** — the value it replaces is a stale copy of the *same computation*,
  never human-composed prose.
- **Reversible** — via ADR-14's revision log, so the write is undoable by inspection.

Anything failing any clause stays permanently hand-authored. No confidence score makes prose
eligible — the eligibility test is a closed set of clauses, not a threshold to tune.

**One computation exists today, Tier 1**: recompute `Embodiment` from `Realized-by`'s categorized
locators (ADR-18). `Embodiment` itself is explicitly excluded from *auto-repair* despite satisfying
every clause: a stated-exceeds-computed mismatch could be "resolved" by lowering the claim, which is
the mechanical answer and the wrong one whenever the evidence is merely missing rather than the claim
being wrong (RFC-12 §5). `fix` therefore *reports* the Embodiment mismatch (this is
`check`'s own `embodiment.consistency` rule, surfaced here in repair-shaped form) but does not
silently downgrade a stated value on `--apply` without the record being explicitly selected.

Two further tiers are named, not built: completing a half-stated relationship additively (Tier 2),
and repointing a stale locator only when the move is unambiguously recoverable (Tier 3). Each ships
only once a real case demands it, in increasing order of nerve required.

## Detect mode (default, read-only, CI-safe)

`urzua fix` with no flags: for every record with a stated `Embodiment` and a `Realized-by`, computes
the expected tier and reports a disagreement as a structured entry:

```json
{ "record": "docs/adr/ADR-1-x.md", "field": "Embodiment", "line": 3, "current_value": "...", "computed_value": "...", "tier": 1, "evidence": "Realized-by: code:src/a.rs" }
```

A record whose `Realized-by` locator changed since the `Realized-by` line was last touched (ADR-32)
computes to `Drift detected` regardless of the locators' tier, exactly as `check`'s
`embodiment.consistency` does: `fix` reads the same drifted-record set `check` computes, so the two
never disagree. The repair is written like any other Tier 1 repair, and a stated `Drift detected`
that is no longer drifted (the `Realized-by` line was touched again) is repaired back to the
tier-computed value. `fix` computes drift only when `embodiment.consistency` is enabled, as `check`
does (`MILE-159`): with the rule off or undeclared there is no embodiment verdict to repair, so `fix`
reports it could not run instead of computing a verdict from an empty drift set. It applies the same opt-in `@sha256:` locator pins and
`drift_grace_days` window `check` does (`MILE-129`), so the two still never disagree; `fix` repairs `Embodiment` only; a pin is re-set
by `--rebaseline` (below), never by a Tier 1 repair.

Git trouble is never read as "no drift" (`MILE-159`). A shallow clone, detected once per run, and any
git call that fails while computing drift (a commit the repository does not have, say) are reported as
`notices` with subject `embodiment-drift-git`, in `check` and in `fix` alike; the affected records are
not judged for drift. The per-record `git blame` calls run on a few threads.

### Scoping to paths

`urzua fix [PATHS...]` examines, computes drift for and repairs only the records the paths select
(`MILE-159`). A path is a record file or a directory; a record is selected when its repo-relative
path equals the path or sits under it. Paths are relative to the working directory or absolute. The
population counts the selected records only, and `git blame` and the drift computation run for those
records alone, so a single-record run costs a fraction of a whole-corpus run. The corpus a
back-pointer resolves against stays whole.

A path that selects no record, or lies outside the repository, is reported in `failed` naming it
(`partial-failure`, exit 1), never skipped. In apply mode `--ids` and `--force` select among the
scoped repairs only, so an `--ids` entry outside the given paths matches no pending repair and fails
as above.

In detect mode `--ids` filters the report the same way: only the named records are listed, and an entry
that matches no pending repair is a failure naming it (`partial-failure`, exit 1). `--force`, `--by` and
`--actor` belong to a write, so without `--apply` they are a usage error (exit 2) rather than silently
ignored.

### Judging proposed content (`--stdin-path`)

`urzua fix --stdin-path <path>` reads the proposed content of one record from stdin and judges it
exactly as it would the file at `<path>`, without writing anything (`MILE-159`). It is the entry
point for an editor hook that wants a verdict before the file is saved; the hook that
uses it is described under "The record-edit hook" below.

- **The path need not exist on disk.** A new record is judged from the proposal alone. The record
  type is the one whose `dir` directly holds the path, under the same filename rules discovery
  applies; a path no type governs is `not-run` (exit 2) with an `error` naming the reason, never an
  empty `ok`.
- **The proposal replaces the on-disk record** for this run. Nothing else is examined, and drift is
  computed for this one record only: the `Realized-by` line is blamed against the proposed text, so
  an unchanged line keeps its commit, and a line the proposal edits or adds counts as touched now
  (no locator can have moved since). A path with no git history is not drifted.
- **The output is the ordinary report**, and `repairs` is its stable machine-readable list for that
  one path: each entry is `record`, `field`, `line` (the 1-based line of the stated field in the
  proposed content), `current_value`, `computed_value`, `tier` and `evidence`. A hook reads
  `status` (`repairs-available`, `ok`, or `not-run` when the record states no `Embodiment` to
  judge) and `repairs`; `line` is the location to point at. `notices` carry git trouble as in the
  whole-corpus run.
- **It cannot be combined** with `PATHS`, `--apply`, `--ids`, `--by` or `--force`: judging proposed
  content never writes, and the combination is `not-run` (exit 2) rather than a silently ignored
  flag.

### The record-edit hook

`urzua harness run edit` (`MILE-178`, `ADR-69`, `MILE-191`, `ADR-78`, `MILE-198`, `ADR-89`; the request
and verdict are in `SPEC-24`) is the harness-neutral core that judges a record edit before it is written,
the way `urzua check --stdin-path` (`SPEC-2`) does, so the editor judges every enabled rule on the edited
record and not only the computed fields `fix` repairs. Its contract: stdin is a request carrying the
proposed full content and the record path (absolute or repo-relative); stdout is one JSON object
`{verdict, path, message, findings, override?}`; exit 0 lets the edit proceed and 1 blocks it. It holds no
rule of its own: it reports the findings of the enabled rules.

- `block`: by default (`block_on: introduced`, `SPEC-3`), an unwaived error finding the edit introduced. An error is introduced when the record as it
  is on disk, judged the same way, does not already report it (same rule and message). The message
  lists each as `file:line: [rule] message`. A `run-command` suggested action's command is shown only for an error already on disk or a warning, never for one this edit introduced, since the command repairs the record on disk and answers `matched no pending repair` for a proposed edit.
  An error already on disk is listed as "Already failing on disk", not blocked, so the edit that fixes
  something else is not refused. A baseline that cannot be judged leaves every error introduced. With
  the configuration's top-level `block_on: all` (`MILE-195`), `block` is any unwaived error in the
  proposed content, including one the record on disk already has; the baseline run is skipped and nothing
  is listed as already failing. The named override and its log behave the same under both values. A
  `block_on` value other than `introduced` or `all` is `allow-unjudged`, and the message names it.
- `allow-with-warnings`: judged; the warnings (and any error already on disk) are in `message` and the
  Claude Code adapter shows them to the agent. Nothing blocks.
- `allow`: judged, no findings. The message counts the rules that ran and names each enabled rule
  that cannot judge a single-record edit (`not-applicable` in the report), so an agent is never told
  "every rule" when some were not run. Those rules run at commit and in CI.
- `allow-unjudged`: urzua is missing, answers `not-run` (including a configuration in which no enabled
  rule can look at the record), or exits otherwise than 0 or 1, or prints non-JSON. The edit proceeds and the message says plainly that it was not judged. An `embodiment-drift-git` notice (a shallow clone, a failing git call) is not one of these: the hook still judges every finding that needs no history and blocks on them, and the message names the drift rule it could not judge (`allow-with-warnings` when nothing else is found).
- `not-governed`: urzua reports that no record type governs the path (silent, the file is not a record).
- `allow-override` / `block`: an override named in the request (or `URZUA_HOOK_OVERRIDE=NAME`) lets a block through and
  puts the name in the output; an override without a name keeps the block. When the configuration
  declares `override_log` (`SPEC-3`, `ADR-78` item 1, `MILE-188`), the override also needs a reason
  (a reason in the request or `URZUA_HOOK_REASON=TEXT`; the Claude Code translator passes the environment) and the
  hook appends one row to that committed log for each finding it let through: date, name, record, field
  (the field a finding's suggested action targets, else `-`), rule, reason, and a blank `Reviewed-by`
  for a person to fill in. `override.unreviewed` (`SPEC-2`) then reports the row until they do. A log
  that cannot be written blocks the edit, since an override that is not recorded is not allowed. With no
  `override_log` declared the override is let through and the message says it was NOT recorded. The core
  reads the configuration's top-level `override_log:` and `block_on:` scalars for this.

The Claude Code translator `.claude/hooks/urzua-claude-code.py`, installed by `urzua harness install`
(`SPEC-19`) and wired in `.claude/settings.json` for `Write|Edit|MultiEdit`, rebuilds the proposed
content from the payload and maps the verdict to Claude Code's hook output; a second harness is a second
adapter over the same core. Measured on this corpus before the core moved into the engine (`MILE-191`,
release build, 522 records, best of 20 runs on a loaded 4-core machine, through the shell chain): about
0.43s per record when the edit has no errors, which is one `check --stdin-path` run, and about 0.77s when
it has errors, which pays for the second, baseline run. Compiling each pending pattern once per process, instead of on every field, took a single run
from about 0.55s to about 0.42s. `fix --stdin-path` alone is about 0.2s.

`--tier` defaults to `1` and is currently locked there — no other tier is implemented, so accepting
a different value would silently do nothing rather than fail loud. Writes nothing, ever. Exit
non-blocking.

### The `Status` repair (`MILE-183`)

A second Tier 1 repair exists, for a type that declares `statuses.delivery` (`SPEC-3`) with
`status.delivery-agreement` or `embodiment.consistency` enabled. When a record's evidence (`Realized-by`
locators and source back-pointers, drift ignored) computes to a declared delivered tier while its
`Status` is a declared not-done status, `fix` proposes the declared `set_status` as a repair with
`field: Status`, and `--apply` writes it through the same path and structural revision row as the
`Embodiment` repair. The one correct value is computable only in this direction. A success `Status`
with no evidence is not downgraded: `fix` reports it as a notice naming the record, since the repair is
adding evidence. `fix` computes no drift for this repair, so it runs when only
`status.delivery-agreement` is enabled.

A type that declares `statuses.delivery.items` (`SPEC-3`, `ADR-96`) has its `Status` computed from the
Done-means items, and `fix` writes only what the engine computes: `set_status` when every item that is
not waived resolves (and at least one at a delivered tier), and the declared `partial_status` when some
resolve and the record sits at a not-done status short of it. The repair's revision row cites the
counts ("recomputed `Status` from `the Done means items (3 of 5 items resolved)`"), so the attribution is
the evidence. `fix` never downgrades a `success` status: a finished record with an unresolved item is
`status.delivery-agreement`'s `DoneWithUnresolvedItems` error, which `check` always fails on, and `fix`
reports a notice naming the unresolved items; the repair is evidence, an inline waiver, or reopening by
hand. `fix` does not write `partial_status` to a record that is derived-blocked (a `progress` status
would contradict `sequence.status-agreement`): the finding stays and names the open dependency. A pin
inside an item's evidence clause is re-pinned by `--rebaseline` as one in the header is.

### How `check` points at these repairs (`ADR-73`, `MILE-44`)

A `check` finding of `embodiment.consistency` or `status.delivery-agreement` whose repair `fix` writes
carries a `suggested_action` of kind `run-command` with `argv` `["urzua", "fix", "--apply", "--ids",
"<record path>"]` and no `value`: the repair stays the one place the computed value lives. The
command is offered only when `fix` would list a repair for that record and field, so a finding `fix`
does not repair (a stated tier with neither a locator nor a citing back-pointer, a success `Status`
with no evidence) offers an edit to the field or nothing. A test runs each offered command and checks
the finding is gone. The command is the whole promise: `--apply` still needs an identity (`--by`) and
its own gates. The editor hook (`ADR-69`) now relays `check --stdin-path` and reads its findings (`ADR-78`).

## Apply mode (`--apply`)

Every one of the following is a hard gate — none are optional, none are defaults-on:

- **`--ids <record,...>` or `--force` is required.** Nothing applies without one; `--force` (apply to
  every detected repair) is never implied by `--apply` alone.
- **An `--ids` entry that matches no pending repair is a failure.** It is reported in `failed` naming
  the entry (`partial-failure`, exit 1) and writes nothing, never a clean `ok` (`BUG-166`). An entry is
  matched against a record's repo-relative path; accepting a record id is not defined here.
- **Identity is required**, resolved via `urzua-io::resolve_identity_with` under the configuration's
  `attestation` (`SPEC-3`, `ADR-89` section 5). Under `identity: gh-verified`, the default and what an
  absent key means, `gh api user` wins even over an explicit `--by`, which wins over `git config
  user.name` (`ADR-31`'s amendment); with `require_verified: true` and no `gh` answer the command is
  `not-run` (exit 2) and writes nothing, the lock included. Under `identity: asserted`, `--by` is taken as
  given and the verified login, when `gh` answers, is recorded beside it. None resolving is a hard error,
  never a placeholder author. An identity containing `|` or a control character (a line break) is refused,
  whichever tier supplied it, because it is written into a revision-log table row (`--reason` is refused
  for the same reason). A `--by` that a verified `gh` login replaced surfaces as the report's
  `identity.replaced_by` and as a `notices` entry (`ADR-46`), never silently overridden or discarded.
- **The report names the identity.** An apply report carries `identity`: `name`, `source`
  (`gh-verified`, `asserted` or `git-config`), `actor` (`agent`, `human` or `unspecified`), and
  `replaced_by` and `verified_login` when they apply. Detect mode carries none.
- **`--actor agent|human`** states who ran the command and is recorded in the row; a run without it
  records `unspecified`, and any other value is `not-run`. It is the caller's claim and not proof, with the
  standing of `--by` (a verified login names the account whose token ran the command, not whether a
  person or an agent held it: `RFC-2`, `BUG-16`). An agent passes `--actor agent` (`AGENTS.md`); the lifecycle
  commands `ADR-89` section 1 adds (`MILE-199`) are to pass it when built.
- **A missing Revision log section refuses that record's write outright, where the log is required.**
  A type declares `revision_log: {required: false}` (`MILE-159`, SPEC-3) to be written without a row;
  `true` is the default. The error names the record and the identity looked for. ADR-14's reversibility
  clause is enforced per write, not assumed — a repair with nowhere to record itself is reported
  failed, and every other selected repair still proceeds independently. Partial failure is a
  first-class outcome, not a batch-wide abort.
- **Re-verification at write time.** The exact file is re-read from disk immediately before writing;
  the pure `apply_repair` function itself fails if the value it expects to replace is no longer
  present — the shape a concurrent edit since detect ran would take.
- **A corpus-level lock** (`.urzua/cache/fix.lock`, atomic `create_new`) serializes concurrent
  `--apply` invocations; a second process fails loud rather than interleaving writes. No stale-lock
  cleanup exists yet — a crashed process leaves the lock file behind, requiring manual removal. A
  real, named gap, not silently accepted as fine.
- **A write replaces the file whole or not at all.** The new text is written to a temporary file in
  the record's directory and renamed over it, keeping the mode, so a full disk or a killed process
  leaves the old record, never an empty or truncated one. A read-only record is refused.
- **The edit touches only the one field's line.** Every other field and all prose is untouched,
  byte-for-byte — verified by test, not just intended.
  That includes the file's line endings and trailing-newline state, and the new revision-log row
  repeats the existing table's own row prefix (`BUG-164`, `BUG-165`).
- **The value is replaced by position, never by substring** (`BUG-192`). The edit rewrites what follows
  `Field:` on the field's own line and nothing else. A stated value that is empty, `~` or `null` is
  filled in place, and its revision row reads "stated no value disagreed" rather than quoting an empty
  span.

- **A status an `authority` gate names is never written** (`ADR-70` amendment of 2026-11-05, `ADR-89`
  section 4, `MILE-199`). When the value `fix --apply` computes for a type's status field is one a gate of
  the top-level `authority` table names for that type, the repair is refused and reported in `failed` with
  the reason, and the record is left as it was: such a status is a decider's, not evidence's. The lifecycle
  command `/done` runs `fix --apply` and so inherits the refusal.

Every applied write appends a revision-log entry of the type's `tool_class` (`structural` unless
the type declares another, `ADR-94`) naming the computation and the evidence read, so the write is
reversible both by `git revert` and by reading the corpus itself. The row is laid out by the log's
header: the date in the date column (the first when `date_column` is undeclared), the class in the
class column (the last when `class_column` is undeclared), the narrative in the first remaining
column and any other cell empty. A log with no such column left, or whose declared column is
missing or repeated, is refused like one with no rows to append after.

The entry names who wrote it, where the name came from and who held the command (`ADR-89` section 5):
`Tool-authored (by NAME, identity: SOURCE, actor: ACTOR): recomputed ...`, with `, verified login: LOGIN`
before the closing parenthesis when an `asserted` name has a verified login beside it. Rows written
before the identity source existed read `Tool-authored (by NAME): ...` and are left as they are.

## Re-baselining pinned evidence (`--rebaseline`)

`urzua fix --rebaseline` is the review-then-apply refresh `RFC-2` proposed, for the opt-in
`@sha256:` pins of `MILE-129` (`MILE-181`). It replaces the Tier 1 repair for that run and takes
the same `PATHS` scoping.

- **Detect** (`--rebaseline`, read-only): `rebaselines` lists each record with a pinned locator whose
  current content hash differs, as `record`, `field`, `line` and `pins`, each pin carrying `locator`,
  `recorded_hash` and `current_hash`. Status is `repairs-available` while any is listed. A locator
  whose file cannot be read is not a stale pin (`embodiment.locator-exists` reports it).
- **Apply** (`--rebaseline --apply`): the gates of Tier 1 apply hold unchanged (`--ids` or `--force`,
  never implied; an `--ids` entry matching nothing fails; identity as above; lock; re-read before
  write; a missing revision log refuses that record). `--reason <text>` is required too: one line
  without `|` or a line break (`\n` or `\r`), because an attestation with no reason is the rubber stamp
  `RFC-2` set out to prevent. The pin hashes are computed after the lock is taken, so evidence edited
  while the command started is pinned as it is then.
- **What is written.** For each chosen record, every stale pin on the `Realized-by` line becomes the
  current hash and nothing else on the line, in the file or in any other record, moves. One
  `structural` revision row names who attested (`RFC-25`: attribution, not cryptographic
  attestation), the identity source and the actor kind, the old and new hash prefixes and the reason:
  `Re-baselined (attested by NAME, identity: SOURCE, actor: ACTOR): re-pinned ...`, with the same
  optional `, verified login: LOGIN`. Earlier rows, `Re-baselined (attested by NAME): ...`, are left
  as they are.
- **The record is the selector.** All of a record's stale pins move together; per-locator selection
  is not offered. A second run finds nothing to re-pin.
- **Scope of the baseline.** Only pinned locators are re-baselined. Drift on an unpinned locator
  compares against the commit that last touched the `Realized-by` line, which `fix` cannot move
  without editing the line, so it is cleared by pinning the locator by hand (rewriting a pin touches
  the line, which also resets the baseline of the record's unpinned locators once committed).
  `--rebaseline` does not repair `Embodiment`: run a plain `urzua fix` afterwards for a stated
  `Drift detected` that is no longer drifted.
- **Scoped pins** (`ADR-74`, `SPEC-3`). A pin on `path#scope` is listed and re-pinned like any
  other, from the hash of the span its scope resolves to now, and the edit keeps the scope in the
  spelling as written. Each listed pin of a scoped locator carries `span` (`start`, `end` and the
  pinned `text`), so the reviewer diffs exactly those lines and sees that the resolver found the
  item the decision is about; the revision row names the lines. A scoped pin that does not resolve
  (renamed, deleted, ambiguous, no resolver, range past the end of the file, file missing or unreadable) is never re-pinned,
  because re-hashing whatever is there now would turn a deleted symbol into a silent pass. It is
  listed under `failed_pins` (`record`, `locator`, `reason`), per pin and not per record, status is
  `partial-failure`, and the record's other stale pins still move. A rename is repaired by editing
  the scope on the `Realized-by` line.
- **Directory and glob pins** (`BUG-176`, `SPEC-3`). A pin on a directory or glob whose kind the
  repository declares in `locator_kinds` and `pinnable` is re-pinned from the hash over the sorted
  (path, file content hash) list of the files it covers now, and each listed pin carries `files`, so
  the reviewer sees which files the attestation covers; a file added to or removed from the set
  changes the pin like an edit does.
- **Authoring route.** A first hash for a new scoped pin comes from `urzua explain <path>#<scope>`
  (`SPEC-13`), which prints the span and the locator ready to paste. A bogus placeholder hash
  re-baselined later would record a re-verification of something never verified, and is not a
  supported route. To reproduce a hash by hand from a span of lines `a` to `b` of a file with no BOM
  and no bare `\r`: `sed -n 'a,bp' file | tr -d '\r' | sed -z 's/\n$//' | sha256sum`.
- It cannot be combined with `--stdin-path`, and `--reason` without `--rebaseline` is `not-run`.

## Renewing a claim (`urzua reverify`, `ADR-97`)

`urzua reverify` is the one attributed way to renew a claim that `embodiment.claim-expired`
(`SPEC-3`) reports. It follows `fix --rebaseline`: review first, then apply. It needs that rule
declared and is `not-run` otherwise.

- **Two acts, by design.** `reverify` never moves an existing pin: it re-checks that each pin still
  matches and dates the claim. A changed pin is moved by `fix --rebaseline`, with its own reason,
  and only then does `reverify` renew. The one hash it writes is a *first* pin on a locator that has
  none, because a claim is not renewed over evidence that only git history vouches for.
- **Detect** (`urzua reverify [--ids <record>,...]`, read only). With no `--ids` it lists the
  records whose claim has expired; with `--pin-only`, those with an unpinned locator to pin; with
  `--ids`, any record of a listed type, expired or not. Each record carries `finding` (the expiry
  message), `age_days`, `period_days`, and `locators`: per locator `locator`, `category`, `resolved`,
  `pinned`, `recorded_hash`, `current_hash`, `check` and, for a scoped one, `span` with its text, so
  the reviewer reads the content they attest. `check` is `hash-checked`, `will-pin`, `history-only`
  (a locator outside `require_pin`, such as `spec:`, re-resolved and judged by git history only),
  `changed` or `unresolved`. `refused` lists why a record cannot be written; with `--pin-only`,
  `commits_since` lists the commits that touched an unpinned locator since the `Realized-by` line
  was last touched. A record whose revision log a row cannot be appended to (no log section, an unreadable
  one, or one with no rows) is listed in `refused` with that reason, because both modes append an attested
  row: detect never offers a write that `--apply` would refuse. Nothing is written.
- **Apply** (`--apply`): the gates of `fix --apply` hold unchanged. `--ids` or `--force`, never
  implied; an `--ids` entry matching nothing fails; identity resolves under `attestation` (`--by`,
  the identity source, `--actor agent|human`); a lock; the pin hashes are read after the lock; a
  record with no revision log is refused. `--reason` is required, one line without `|` or a line
  break. `--apply` names at most `max_batch` records (a longer `--ids` list is `not-run` and names the
  cap, nothing is truncated), every row of a batch carries the same `--reason`, and the report prints
  it once. `--force` lifts the `max_batch` cap for one record and is refused when it selects more than one; with
  `--ids` it renews the named record only, never every expired claim.
- **Refused, writing nothing for that record:** a locator that does not resolve; a pinned locator
  whose hash differs (run `fix --rebaseline`); drift by the git-history rule, inside
  `drift_grace_days` too (attesting over a file that changed and was not re-read is the rubber stamp
  this command exists to prevent); a stated `Embodiment` that `embodiment.consistency` would report;
  a record the expiry rule does not judge (neither a `Realized-by` value nor a Done-means item evidence clause) or cannot date (no valid anchor date and no commit for its first evidence line: the `Realized-by` line, or the first item evidence clause); a rule with no `anchor_field`; a record with
  no usable revision log; and any actor but `--actor human` (`agent`, or none given) where the matching
  entry's level is `error`.
- **The pin step.** Every locator of a `require_pin` category without a pin is pinned first, in the
  locator's own spelling (file-level or scoped exactly as written, a scope is never invented) with
  `@sha256:` and 64 lowercase hex of the content the tool just read (a file the repository tracks and that sits
  inside it: a path outside the repository, or one git does not track, does not resolve and refuses the
  record, as `embodiment.locator-exists` judges it), on the `Realized-by` line and in
  Done-means item clauses. A `spec:` locator is not pinned: it cites a record with its own lifecycle,
  is re-resolved, and the row counts it apart. A first pin is written on the exact locator only: the pin
  for `a.rs#Foo` is never spliced into `a.rs#Foo::bar` on the same line.
- **`--pin-only`** pins the unpinned locators of the chosen records, allows the drift, writes the
  `Pinned` row and sets no date, so nothing is renewed: it is how an unpinned locator that drifted is
  pinned (`fix --rebaseline` has nothing to move there), and the one-off migration of a corpus. It
  refuses a pinned locator whose hash differs and a record with nothing to pin, never doing nothing
  silently. Its reason names the real delta, as a re-pin's does.
- **What is written.** `anchor_field` is set to today (UTC), added to the header when absent.
  `Status` and `Embodiment` are not touched: a reverify dates a claim and never lifts it. Revision rows
  of the type's `tool_class`:
  `Pinned (attested by NAME, identity: SOURCE, actor: ACTOR): N locators pinned, F file-level, Q scoped. Evidence: <locator>@sha256:<12 hex>, ... Reason: <reason> **Why:** a claim is renewed only over pinned code and test evidence.`
  and
  `Re-verified (attested by NAME, identity: SOURCE, actor: ACTOR): N locators re-resolved, P hash-checked, S spec locators checked by git history only. Evidence: ... Reason: <reason> **Why:** the claim was <age> days old against a <days>-day period.`
  A record pinned and renewed in one run gets both rows, in that order, with the one reason. A
  repository with no `require_pin` gets `H unpinned locators checked by git history only` for `S spec
  locators ...`.
- **Who may renew.** An agent may renew and may pin when an independent reviewer reads the pull
  request's `Pinned:` and `Re-verified:` lines and confirms each reason matches what was read
  (the re-pin decision of 2026-11-05); the engine records the actor and cannot verify the reviewer.
- **What proves a renewal.** The tool proves that every locator resolves, every pin still matches,
  every `require_pin` locator is pinned, and the record agrees with its computed `Embodiment`. It
  records who attested and what it saw. It does not prove that a person read the code: the reason is
  the reviewer's judgement.
- The exit codes are `fix`'s: 0 for a review or an applied batch, 1 for a refused or failed record,
  2 for `not-run`.

## Purity boundary

`urzua-core::fix::apply_repair` is a pure `&str -> Result<String, String>` transform. The CLI
(`urzua-cli`, via `urzua-io`) performs the actual file read, lock acquisition, identity resolution,
and write. `urzua-core`'s purity test (ADR-5/6) must pass with `fix`'s dependencies in place —
asserted by test, not assumed by convention.

## What's deliberately not built

- **Tiers 2 and 3** — additive relationship completion and locator repointing. Ship only once a
  real case needs them.
- **Stale-lock cleanup** — a crashed `--apply` process's lock file requires manual removal today.
- **`migrate schema --apply`'s delegation to `fix`** — named as a consequence of apply mode existing
  (ADR-20), not yet wired; blocked on deciding which newly-required fields are ever tool-writable
  (structurally, almost none — see SPEC-14).

## References

- ADR-15 — the eligibility test and tier model.
- ADR-18 — the one Tier-1 computation (Embodiment).
- ADR-19 — detect mode shipped alone, apply mode's three then-missing dependencies named.
- ADR-20 — apply mode's hard gates, once those dependencies existed.
- ADR-14 — the revision-log shape every applied write appends to.
- RFC-2 §2 — the identity-resolution tiering apply mode reuses.
- ADR-89 section 5 — the configurable identity source and the actor kind in the row; BUG-16 and MILE-66
  — why a verified login is not proof of who acted, and why it beats a typed name by default.
- RFC-12 §5 — why Embodiment is excluded from auto-repair despite satisfying every eligibility clause.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, bundling ADR-15/18/19/20 into one buildable document for the `fix` feature area. **Why:** MILE-77 named this as a real feature area (an eligibility test, one computation, a detect/apply split, and apply's hard gates) that was documented only as four separate ADRs, none of which was a complete build reference on its own. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Corrected the identity-resolution order, stale since ADR-31's amendment (`gh api user` now wins even over `--by`, which wins over `git config user.name` -- not "`--by` wins outright" as this spec still said). Noted the new `notices` field (ADR-46) for a `--by`/`gh` divergence. **Why:** found live, touching this exact paragraph for the `notices` addition -- leaving the identity-order claim wrong while editing the sentence next to it would have been the same "found it, patched around it" mistake this project's own `AGENTS.md` already warns against. | **substantive** |
> | 2026-11-02 | Bumped to `0.5`. Apply mode now fails an `--ids` entry that matches no repair, and keeps CRLF endings, the absent trailing newline and a plain-table log's row prefix. **Why:** round 29 found each as a silent no-op or a whole-file rewrite (`BUG-164`, `BUG-165`, `BUG-166`). | **substantive** |
> | 2026-11-03 | Bumped to `0.6`. `fix` now computes `Drift detected` (ADR-32) from the same drifted-record set `check` uses, so it is detected and, on `--apply`, written like any other Tier 1 repair. **Why:** MILE-15 -- ADR-32 named this as separate follow-up work. | **substantive** |
> | 2026-11-04 | Bumped to `0.7`. Noted that `fix` reads the same opt-in locator pins and grace window as `check` and never rewrites a pin. **Why:** `MILE-129` -- `fix` shares `check`'s drifted-record set, so a new drift input changes what it repairs. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. `fix` computes drift only when `embodiment.consistency` is enabled and reports git failures and shallow clones as `embodiment-drift-git` notices. **Why:** `MILE-159` -- drift ran unconditionally (about 1.1s on this corpus), and git errors read as a clean zero, which is how a shallow clone once produced a false zero. | **substantive** |
> | 2026-11-04 | Bumped to `0.9`. `fix` takes `PATHS` that scope examination, drift and repair, and reports a path matching no record. **Why:** `MILE-159` -- an editor hook needs a quick single-file refresh, and the whole-corpus run is about 0.8s on this corpus against about 0.15s for one record. | **substantive** |
> | 2026-11-04 | Bumped to `0.10`. `fix --stdin-path <path>` judges proposed content for one record without writing, for a path that need not exist yet, with drift computed for that record alone; each repair carries the 1-based `line` of the stated field; the detect-mode example now shows the real snake_case shape. **Why:** `MILE-159` -- an editor hook judges a file before it is saved and must find its own repair without parsing the whole list. | **substantive** |
> | 2026-11-04 | Bumped to `0.11`. Apply mode writes without a revision-log row for a type declaring `revision_log: {required: false}`; a required log (the default) still refuses a record with no section, now naming the identity looked for. **Why:** `MILE-159` -- an adopter's records have no revision log section, so `--apply` could never run on them. | **substantive** |
> | 2026-11-04 | Bumped to `0.12`. `fix` proposes, and `--apply` writes, the declared success `Status` for a record whose evidence computes to a declared delivered tier while its `Status` is declared not done (`statuses.delivery`); a success `Status` with no evidence is reported as a notice and never downgraded; the revision row now names the computed value; `fix` runs when only `status.delivery-agreement` is enabled. **Why:** `MILE-183`. | **structural** |
> | 2026-11-04 | Bumped to `0.13`. Documents the record-edit hook that relays `--stdin-path` verdicts to a harness, replacing the example editor script. **Why:** `MILE-178`, `ADR-69`. | **substantive** |
> | 2026-11-04 | Bumped to `0.14`. `fix --rebaseline` lists stale `@sha256:` pins and, on `--apply --ids`/`--force --by --reason`, re-pins the chosen records with an attributed revision row. **Why:** `MILE-181` -- a pin was a block with no sanctioned way to say "re-verified, still holds"; `RFC-2` proposed exactly this review-before-apply refresh. | **substantive** |
> | 2026-11-04 | Bumped to `0.15`. Documents how `check` findings point at the `Embodiment` and `Status` repairs: a `run-command` `suggested_action` naming `urzua fix --apply --ids <record>`, offered only where `fix` lists that repair. No change to `fix` itself. **Why:** `MILE-44`, `ADR-73`. | **substantive** |
> | 2026-11-05 | Bumped to `0.16`. Apply replaces a field's value by position after `Field:`, never by substring, and fills an empty, `~` or `null` stated value in place; the revision row says "stated no value" for it. **Why:** `BUG-192` -- an empty stated value matched everywhere and the computed value was written at column 0 (`ImplementedEmbodiment: null`). | **substantive** |
> | 2026-11-05 | Bumped to `0.17`. `--rebaseline` re-pins a scoped pin from the hash of its resolved span, keeps the scope in its spelling, shows the span beside the hashes and lists a scoped pin that does not resolve under `failed_pins` instead of re-pinning it; `explain <path>#<scope>` is named as the authoring route. **Why:** `MILE-185` / `ADR-74` -- re-hashing whatever is at a range now would turn a deleted symbol into a silent pass. | **substantive** |
> | 2026-11-05 | Bumped to `0.18`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/fix.rs, code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_paths_integration.rs, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.19`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.20`. The record-edit hook relays `check --stdin-path` and judges every enabled rule that can judge one record, and its `allow` message counts the rules that ran and names the ones it could not judge instead of claiming every rule, with a new `allow-with-warnings` verdict and `findings` replacing `repairs` in its output; `fix --stdin-path` is unchanged and no longer the hook's call. **Why:** `MILE-191`, `ADR-78` item 2. | **substantive** |
> | 2026-11-05 | Bumped to `0.21`. The record-edit hook records a named override in the declared `override_log` (a reason is then required) and blocks an override it cannot record. **Why:** `MILE-188`, `ADR-78` item 1. | **substantive** |
> | 2026-11-05 | Bumped to `0.22`. `--rebaseline` lists and re-pins a directory or glob pin of a kind the repository declares pinnable, each listed pin carrying the `files` it covers. **Why:** `BUG-176` -- a pin on a tree is attested over the files it covers, which the reviewer must be able to read. | **substantive** |
> | 2026-11-05 | Bumped to `0.23`. The record-edit hook reads `block_on: introduced \| all`: `all` blocks on any unwaived error in the proposed content and skips the baseline run; the default and an absent key keep today's verdicts and messages; an unrecognised value is `allow-unjudged` naming the value; the Claude Code adapter passes the verdict through unchanged. **Why:** `MILE-195`, `ADR-78` amendment of 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.24`. `--apply` and `--rebaseline --apply` refuse an identity (`--by`, `git config user.name`, the `gh` login) containing `\|` or a line break instead of writing a row with an extra cell or a split row, and write each record through a temporary file and a rename instead of truncating it first. **Why:** review of the CLI found both: a crafted `--by` corrupted the revision log, and a failed write could leave a record empty (`BUG-192` class). | **substantive** |
> | 2026-11-05 | Bumped to `0.25`. The record-edit hook no longer answers `allow-unjudged` for an `embodiment-drift-git` notice: it judges and blocks on what needs no history and discloses the skipped drift rule; it omits the `run:` repair hint from findings the edit introduced. **Why:** Phase 0 hardening from an adopter switchover -- every edit in a shallow clone went unjudged, and the hint answered `matched no pending repair`. | **substantive** |
> | 2026-11-06 | Bumped to `0.26`. `--apply` and `--rebaseline --apply` resolve identity under the configuration's `attestation` (`SPEC-3`): `require_verified` makes a missing `gh` answer `not-run` with nothing written, `asserted` takes `--by` as given beside the verified login; the report gains `identity` (name, source, actor, the replaced `--by`); `--actor agent\|human` is accepted; and every tool-authored row names the identity source and the actor kind. **Why:** `MILE-201`, `ADR-89` section 5. | **substantive** |
> | 2026-11-06 | Bumped to `0.27`. `fix --apply` refuses to write a status an `authority` gate names for the record's type and reports it in `failed`. **Why:** `MILE-199`, the `ADR-70` amendment of 2026-11-05: a lifecycle command sets a status only when the engine computes it, never one reserved to a decider. | **substantive** |
> | 2026-11-06 | Bumped to `0.28`. The record-edit hook section describes the engine core `urzua harness run edit` and the installed translator, not the deleted `scripts/hooks` shell and Python chain; the verdicts are unchanged. **Why:** `MILE-204` deletes the scripts, `ADR-89` section 6. | **substantive** |
> | 2026-11-07 | Bumped to `0.29`. `--reason` also refuses a carriage return; `--rebaseline --apply` reads the evidence after taking the lock, and every record write is confined to the repository. **Why:** a Phase 0 hardening review found a `\r` reaching the revision-log row, evidence hashed before the lock, and writes that followed a symlink out of the repository. | **substantive** |
> | 2026-11-07 | Bumped to `0.30`. Detect mode (no `--apply`) honours `--ids`: the report lists only the named records and an `--ids` entry that matches no pending repair (no stale pin, under `--rebaseline`) is a `partial-failure`, as under `--apply`; `--force`, `--by` and `--actor` without `--apply` are usage errors (exit 2) instead of being ignored. **Why:** maintainer decision of 2026-11-07, from the CLI review. | **substantive** |
> | 2026-11-07 | Bumped to `0.31`. The `Status` repair for a type that declares `statuses.delivery.items`: `fix` computes from the Done-means items, writes `set_status` or `partial_status` with the item counts in the row, never downgrades a `success` status, writes no `progress` status to a derived-blocked record, and `--rebaseline` re-pins a pin inside an item clause. **Why:** `MILE-214`, `ADR-96`. | **substantive** |
> | 2026-11-07 | Bumped to `0.32`. `fix --apply` and `fix --rebaseline` write the type's `revision_log.tool_class` (default `structural`) in place of the literal, laid out by the log's header: the date in the date column, the class in the class column, the narrative in the first remaining column, any other cell empty; a log with no column left for the narrative, or whose declared column is missing or repeated, is refused. **Why:** `MILE-213`, the build of `ADR-94` and the 2026-11-07 amendment to `ADR-15`; a vocabulary without `structural` would have the tool write a row its own rule rejects. | **substantive** |
> | 2026-11-07 | Bumped to `0.33`. Documents `urzua reverify` (`ADR-97`, `MILE-129`): the review report, the refusals, the pin step and `--pin-only`, the batch cap and the one shared reason, the actor rule, and the `Pinned` and `Re-verified` revision rows. **Why:** `MILE-129`, the build of `ADR-97`. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 5 locators pinned, 5 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/fix.rs@sha256:04e1ba443038, rust/crates/urzua-core/src/fix.rs@sha256:c9cd87e7b00e, rust/crates/urzua-cli/tests/fix_paths_integration.rs@sha256:73cb06a1782f, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs@sha256:27c91b75ad61, rust/crates/urzua-cli/tests/fix_drift_integration.rs@sha256:ff69343ca324 Reason: Pinned the files as they are now after checking each against the record it evidences: init.rs and check_integration.rs still implement and test urzua init adopt mode (SPEC-5), status_model.rs, the own-config test and status_delivery_integration.rs still implement the milestone type and its delivery status (SPEC-6), both fix.rs files and the paths, rebaseline and drift integration tests still implement urzua fix (SPEC-8), and config.yaml and the own-config tests still declare the bug type (SPEC-9). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 04e1ba443038 -> 23e5e6b703b5. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. fix.rs: with_locator_env now takes the records, which fix passes (one added argument, reformatted), and apply and rebaseline are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.34`. The pins on this spec's `Realized-by` locators were re-baselined against the files as they are after the sharded CI and the milestone `Kind` amendment landed on main (the `Re-baselined` row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97`: every stale pin is re-pinned with its real delta before expiry is enabled. | **structural** |
> | 2026-11-08 | Bumped to `0.35`. `urzua reverify` hardening: a locator outside the repository or untracked no longer resolves for a pin; a first pin is no longer spliced into a longer anchored locator (`a.rs#Foo::bar`); any actor but human is refused where the period's level is `error`; `--force --ids` renews the named record; detect lists a record with no usable revision log as refused. Documents the four. **Why:** the round-4 review findings on `reverify` (`ADR-97`); no change to what an accepted run writes. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 23e5e6b703b5 -> 5a67695c913d, `rust/crates/urzua-core/src/fix.rs` c9cd87e7b00e -> 88c815c0639a. Reason: Read the diff of each pinned file: core fix.rs moved apply_rebaseline's evidence-line lookup and revision-row tail into shared helpers (evidence_edit_lines, append_log_rows, revision_log_refusal) that apply_reverify also calls, with the same rows and error text; cli commands/fix.rs gained validate_reason, back_pointer_evidence, into_failures and identity_notices extracted from fix and reverify with the same behaviour; core reverify.rs calls the helpers and its first-pin matcher no longer splices into a longer anchored locator; cli commands/reverify.rs now refuses a record with no usable revision log in detect, refuses any non-human actor at an error-level period, and honours --ids under --force. Each claim these records make about those files still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.36`. `urzua reverify` lists, dates and renews a milestone whose evidence is only its Done-means item clauses, pinning the item locators; the refusal for a record with no evidence names both sources. **Why:** `ADR-97` amendment of 2026-11-08. | **substantive** |
