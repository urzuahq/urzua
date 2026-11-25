---
Stable-Id: 01M6R3NK5Q0XVK27P5T8FN72BN
Version: '0.83'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/fix.rs@sha256:e7577259ef4e79b7507e488f9ac6bceabd644810757f3020805f64749972f5f3, code:rust/crates/urzua-core/src/fix.rs@sha256:c8c3ad750b4ff30be5b5323be74f8bc014179ea9a8ea1f383f454304f7f95e0b, test:rust/crates/urzua-cli/tests/fix_paths_integration.rs@sha256:efefbffa3dc1992d97ff18582c7a15df6ff789f2f036b4ec3b36bfb910a12ff6, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs@sha256:9b0012ff465ddf87fb9334c03ed56826d5dc8bfae58eaaa36ba24b4798d735ac, test:rust/crates/urzua-cli/tests/fix_drift_integration.rs@sha256:ff69343ca324400bde79c266372f9a213728d4d4ef8202beca6e12e54aa35352
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

Two further tiers were named: completing a half-stated relationship additively (Tier 2), and repointing
a stale locator only when the move is unambiguously recoverable (Tier 3). Tier 3 is not built.

**Tier 2, the missing inverse of a relation (`MILE-287`).** A real case demanded it: an agent could make a
reciprocal change to two records only through the shell. When `relation.pair-reciprocity` or
`relation.supersession-reciprocity` is on and reports that record A claims a relation with B whose inverse
field does not name A, `fix` lists a repair on B (`"tier": 2`) whose `computed_value` is B's field with A's id
added, and an `inverse` object naming the claimant, its field, whether B writes the field yet
(`field_absent`), and `status`, a note that `fix` never changes a `Status` and that a status move the relation
implies (to `Superseded`, say) is a separate step, gated where the authority table names the status. The
repair is read from the rules' own findings, so it covers exactly what `check` reports, in either direction
(a written inverse with no forward claim gets the forward claim), for every claimant of a list-valued field.
It is additive and keeps the form the field is written in: a flow list `[a]` becomes `[a, b]`, a block list
gains a `- b` line at the items' indent, a quoted scalar `"a"` becomes `"a, b"`, a plain one `a, b`, and an
empty, `null`, `~` or `—` value becomes the ids; a value that names no record is never overwritten: it is
reported as a warning notice instead. A target that is a protected path (a path gate of `authority`) is not
written: the notice says a decider makes the paired change, so `fix` never writes what the editor hook would
refuse an agent. A target that does not exist, a target whose type does not declare the field, a claim excluded by a
pair's `types` or `cross_type`, and an acknowledged provisional claim get no repair; the first two are
reported as notices. `--apply --ids <B>` writes it, adding the field's line at the end of the front matter
when absent, with one tool-authored revision-log row naming the claim it answers. Either reciprocity rule
being on is enough for `fix` to run. It is Tier 2, the one Tier 2 repair built: `--tier` still accepts only 1, its
refusal says the Tier 2 repair runs in every run without the flag and that Tier 3 is not built, and the
default command, which the reciprocity findings print, runs both tiers. Decided by the builder on 2026-11-16, pending
maintainer review.

## Detect mode (default, read-only, CI-safe)

`urzua fix` with no flags: for every record with a stated `Embodiment` and a `Realized-by`, computes
the expected tier and reports a disagreement as a structured entry:

```json
{ "record": "docs/adr/ADR-1-x.md", "field": "Embodiment", "line": 3, "current_value": "...", "computed_value": "...", "tier": 1, "evidence": "Realized-by: code:src/a.rs" }
```

A record whose `Realized-by` locator changed since the `Realized-by` line was last touched (ADR-32)
computes to `Drift detected` regardless of the locators' tier, exactly as `check`'s
`embodiment.consistency` does: `fix` reads the same drifted-record set `check` computes, so the two
never disagree. The repair is written like any other Tier 1 repair, except where the drift comes
from a changed pinned file (a pinned locator of the record no longer hashes to its pin, as
`--rebaseline` lists it, and the record computes to the drift state; drift on an unpinned locator
beside a pin that still matches is repaired as before): `fix` does not write the drift state over a stale pin, since that would quiet
`embodiment.consistency` while the pin stays stale. It reports the record in a warning notice,
`embodiment-drift-pinned` (its `detail.records`), names `urzua fix --rebaseline --ids <records>` as
`next_action`, and an `--ids` entry naming it fails with that reason (`MILE-266`, decided by the
builder on 2026-11-16, pending maintainer review). After the re-pin the record computes to its tier
again. A stated `Drift detected`
that is no longer drifted (the `Realized-by` line was touched again) is repaired back to the
tier-computed value. `fix` computes drift only when `embodiment.consistency` is enabled, as `check`
does (`MILE-159`): with the rule off or undeclared there is no embodiment verdict to repair, so `fix`
reports it could not run instead of computing a verdict from an empty drift set. `--rebaseline` (below) needs
neither rule: it compares pin hashes and runs whenever the repository has pins, because it is the fix
`embodiment.pin-stale`, on by default, suggests. It applies the same opt-in `@sha256:` locator pins and
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
  the entry (`partial-failure`, exit 1) and writes nothing, never a clean `ok` (`BUG-166`), and its
  `next_action` is `urzua fix` (`urzua fix --rebaseline` for a re-pin), the dry run that lists the pending
  repairs by path, and `urzua fix --conflicts` for an unmatched `--conflicts` entry (`BUG-207`). An entry is matched against a record's repo-relative path; accepting a
  record id is not defined here. An entry written `./`-prefixed, as an absolute path inside the
  repository or with the OS separator names the same record as its repo-relative form, in every mode that
  takes `--ids` (plain, `--rebaseline`, `--pin`, `--conflicts`), and in `reverify --ids`; an absolute path outside the repository is
  kept as written and fails naming it (`BUG-208`).
- **The exit code says whether the command wrote what it was asked to.** `not-run` is exit 2 in every mode
  (a configuration that fails to load exits as `check` does), `partial-failure` or any `failed` entry is
  exit 1, and `ok`, `repairs-available` and `applied` are exit 0.
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
- **`--actor agent|human`** states who ran the command and is recorded in the row. With `--apply` it is
  required when the resolved identity is `gh-verified` (`fix --apply`, `fix --rebaseline --apply`, and
  `reverify --apply`, `ADR-101`: people own decisions): a verified login is a person's, so a row with no
  actor would read as that person's own attestation of what an agent may have run, and the run is `not-run`
  (exit 2) naming the flag, with nothing written. Under an `asserted` or `git-config` identity a run without
  it still records `unspecified`, and any other value is `not-run`. It is the caller's claim and not proof, with the
  standing of `--by` (a verified login names the account whose token ran the command, not whether a
  person or an agent held it: `RFC-2`, `BUG-16`). An agent passes `--actor agent` (`AGENTS.md`); the lifecycle
  commands `ADR-89` section 1 adds (`MILE-199`) are to pass it when built.
- **A missing Revision log section refuses that record's write outright, where the log is required.**
  A type declares `revision_log: {required: false}` (`MILE-159`, SPEC-3) to be written without a row;
  `true` is the default. The error names the record and the identity looked for. ADR-14's reversibility
  clause is enforced per write, not assumed. A log whose table has its header and separator but no
  row yet takes the tool's row as its first, after the separator, in the table's own prefix (`fix
  --apply`, `fix --rebaseline --apply` and `reverify --apply`, `MILE-267`); a section with no table
  at all is refused. A repair with nowhere to record itself is reported
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
  whose file cannot be read is not a stale pin (`embodiment.locator-exists` reports it); `check` reports the same stale pins on every record, with no `--rebaseline`, as `embodiment.pin-stale` (`SPEC-2`).
- **Apply** (`--rebaseline --apply`): the gates of Tier 1 apply hold unchanged (`--ids` or `--force`,
  never implied; an `--ids` entry matching nothing fails; identity as above; lock; re-read before
  write; a record with no revision log gets one, below). `--reason <text>` is required too: one line
  without `|` or a line break (`\n` or `\r`), because an attestation with no reason is the rubber stamp
  `RFC-2` set out to prevent. The reason must also say something a reviewer can check: at least `attestation.reason.min_words`
  words (runs holding a letter or digit, default 3) and `min_chars` characters (default 12) (`SPEC-3`), so by
  default `x`, `ok` and `...` are refused (exit 2, nothing written). `reverify --apply` holds its reason to the
  same minimum. The pin hashes are computed after the lock is taken, so evidence edited
  while the command started is pinned as it is then.
- **What is written.** For each chosen record, every stale pin on the `Realized-by` line becomes the
  current hash and nothing else on the line, in the file or in any other record, moves. One
  `structural` revision row names who attested (`RFC-25`: attribution, not cryptographic
  attestation), the identity source and the actor kind, the old and new hash prefixes and the reason:
  `Re-baselined (attested by NAME, identity: SOURCE, actor: ACTOR): re-pinned ...`, with the same
  optional `, verified login: LOGIN`. The row ends `**Why:** the pinned evidence changed and ... re-verified that the claim still holds`, naming who re-verified by the actor kind: `an agent`, to be confirmed by a reviewer in the PR, for `agent`; `a person` for `human`; `the attesting actor` for `unspecified`. Earlier rows, `Re-baselined (attested by NAME): ...`, are left
  as they are.
- **A record with no revision log gets one** (`MILE-310`). A re-pin row is an evidence row, so a
  record with no log section is not refused: the write creates the section and the re-pin row is its
  first row. The section is the type's template's own (`.urzua/templates/<type>.md`, its identity
  line through the header's separator row, without the template's rows) when the template has a
  readable one, else one built from the declared `revision_log` identity (a marker as a blockquote, a
  heading as `##`) and the declared date, narrative (`narrative_column`, default `Change`) and class
  columns. It is placed where the template places its log: before the record's first heading that
  the template puts after the log's identity line, set off by blank lines, so `section.order` holds;
  with no template, no log in it or no such heading in the record, at the end after one blank line.
  Detect marks such a record `creates_revision_log: true`, as does the applied entry. Detect and
  apply refuse the same records, writing nothing:
  - a record with no Markdown heading below its header has no body section a log can follow; the
    refusal names it and `next_action` is an `sh -c` command that appends a heading named after the file;
  - a record that ends inside an open code fence, where a new log would read as code; the refusal
    names the fence's line, with no `next_action`;
  - a record a committed copy of which had a log. The copies read: `HEAD`'s, then the copy at the
    merge base of the base ref (`--base <ref>`, default `origin/HEAD`) and `HEAD`, at the record's
    path, at its path before a rename git detects (`git diff -M`), or at the path whose `Stable-Id`
    it carries; when no merge base is visible (a shallow clone) the base ref's own copy is read.
    When those copies show no log, the record's history (`git log --follow -S`, the newest 20
    commits that added or removed the log identity's text) is read too, because a base may be the
    caller's own commit, after the deletion. A run that plans or creates a log reports
    `revision_log_base: {ref, commit, kind}` (`kind` is `remote-tracking` for a ref under
    `refs/remotes/`, `local` for any other, `none` when no base ref resolves), in the listing and the
    apply, and raises a `fix.revision-log-base` warning when no base ref resolves, the base is not
    remote-tracking, no merge base is visible, or the merge base is `HEAD` itself. The row written
    into a created log ends `Revision log created by the tool; base <ref>@<12-hex>.` (or `no base
    ref.`), so the diff shows what was compared. Each record is looked up once per
    run, shared by the listing and the write. In a batch, the first refusal's command becomes the
    report's `next_action`. The refusal names the copy (`<rev>:<path>`) and
    `next_action` is the `git show` that prints it, to restore the log. A `--base` that names no
    commit is `not-run`. A record with no such copy gets its log as above.

  This guard is the tool's refusal only. A log deleted by hand and never re-pinned is not caught by
  it; the durable guard is a diff rule reporting revision-log rows removed between base and head,
  tracked separately.

  `fix --pin` (first pin, `--replace`, `--remove`) plans the log the same way in its listing and its
  apply, with the same refusals and `creates_revision_log`. A write whose row is not an evidence row
  (a Tier 1 or 2 repair, `reverify`) still refuses a record with no log where the log is required.
  *(Placement by the template's order, the `origin/HEAD` default base and the 20-commit history
  fallback, decided by the builder on 2026-11-16, pending maintainer review.)*
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
  spelling as written, a member kind suffix (`Type::name[fn]`, `SPEC-3`) included. An unknown kind
  is a scope that does not resolve, so it is refused like one. Each listed pin of a scoped locator carries `span` (`start`, `end` and the
  pinned `text`), so the reviewer diffs exactly those lines and sees that the resolver found the
  item the decision is about; the revision row names the lines. A scoped pin that does not resolve
  (renamed, deleted, ambiguous, a kind no member entry declares, no resolver, range past the end of the file, file missing or unreadable) is never re-pinned,
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

## Resolving conflicts after a merge (`--conflicts`)

`urzua fix --conflicts` (`MILE-284`, `ADR-67` amendment of 2026-11-16) reads the paths git lists as
unmerged (`git ls-files -u`) during a merge or rebase and judges each that a record type governs. It loads
no records, so a record carrying conflict markers need not parse. It cannot be combined with
`--rebaseline`, `--stdin-path`, `--reason` or `PATHS`; `--ids` narrows it.

- **Detect** (no `--apply`, read-only): `conflicts` lists each unmerged record with `record`, `kind`,
  `outcome` (`resolvable`, `resolved` or `refused`), the `reason` of a refusal, and for a resolvable one
  its `pins` (`locator`, `site`, `base`, `ours`, `theirs`, `merged`, `recomputed`), the `version` it would write
  and `rows_kept`. `site` is the evidence field's name or `Done means item N`. Unmerged files that are not records are listed under `other_unmerged`, never touched.
- **Kind `pin`.** The three index stages are normalised: each pin hash of the evidence field (its key
  line and any indented continuation lines), each pin hash in a Done-means item's evidence clause
  (`ADR-96`; items read with the type's `statuses.delivery.items` declaration), and the type's
  `revision_log.version_bump` field become placeholders, and every revision-log row after the log's
  first is lifted out. A three-way merge of what is left (`git merge-file`) that conflicts means the
  record conflicts in its real content, and it is refused, naming the first conflicting line and, when
  that line belongs to one, the Done-means item. An edit only one side made elsewhere is kept as git
  merged it. A pin in an item clause is handled exactly like one in the evidence field, under the same
  conditions below; but when any item-clause pin moved, the items of the two sides must be identical
  once hashes and pinned locators are masked (each item's text, its evidence clause's other entries and
  order, its waiver clause, and the order of the items), or the record is refused naming the first item
  that differs *(decided by the builder on 2026-11-16, pending maintainer review; the pinned locators
  left out of the comparison by the maintainer's decision of 2026-11-20, see "A locator one side
  changed")*. A pin is matched across
  the stages by its site (the evidence field, or the item's text), its locator and its position among
  that locator's occurrences in the site, so the same locator in `Realized-by` and in an item, in two
  items, or twice in one clause, are separate pins, each restored at its own position. Multi-line
  support covers the header evidence field only: an item clause continued onto a second line has the
  pins of that line left in the text, so if they conflict the record is refused as a content conflict. A row of the base missing from
  either side (removed or rewritten) is refused: a re-pin only adds rows.
- **Row order.** Rows are de-duplicated (rows differing only in whitespace are one row). The side being
  merged into this branch keeps its rows in its own order; each row only this branch has is placed after
  the last row dated on or before it, so within a date the merged-in side's rows come first and this
  branch's own rows last. In a merge or cherry-pick this branch is ours (stage 2) and the merged-in side
  theirs; in a rebase this branch is the commit being replayed (stage 3, `REBASE_HEAD`) and the
  merged-in side is ours, so a merge and a rebase of the same two branches give the same order. Because
  the base's rows need only be present on each side, not at the same positions, a later stop of a
  multi-commit rebase, whose `HEAD` holds the rows an earlier stop interleaved, resolves like the first
  *(decided by the builder on 2026-11-16, pending maintainer review)*.
- **Index stages are the committed records.** When git names the two commits, stage 1 must equal the
  record at a merge base (`git merge-base --all`; in a rebase or cherry-pick, the replayed commit's
  parent), stage 2 the record at our commit and stage 3 the record at their commit, byte for byte, or
  the record is refused: stages written by hand (`git update-index --index-info`) would let the merge
  row claim a re-pin a side never made, or hide one. With several merge bases git's stage 1 is its
  virtual merge of them, which is not read back, so a stage 1 matching none of them is refused *(decided by the builder on 2026-11-16, pending maintainer review)*.
- **A recomputed pin is git's merge of two attested states.** A pin both sides moved to different hashes
  is recomputed only when all of these hold, and the record is refused naming the one that failed:
  the pinned file at our commit (`HEAD`; stage 2 in a merge and a rebase alike) hashes under the pin's
  scope to our side's pin; at their commit (`MERGE_HEAD`, or `REBASE_HEAD` / `CHERRY_PICK_HEAD`) to
  their side's pin; the file is not unmerged; it is a regular file of one mode (`100644` or `100755`) at
  both commits and in the index and not a symlink in the working tree; the working-tree file equals the
  index (stage 0); and the index content equals `git merge-file` of the file at the two commits and their base (the merge base,
  or the replayed commit's parent) *(decided by the builder on 2026-11-16, pending maintainer review)*. The pin is then the hash of the index content. A directory or
  glob pin, or a file not UTF-8 or starting with a BOM under a scoped pin, is refused *(decided by the builder on 2026-11-16, pending maintainer review)*. The refusal
  says the pin needs an ordinary re-pin with a person's reason (`fix --rebaseline --apply ... --reason`).
  When the working-tree file differs from the index only in line endings, the refusal says git's
  line-ending conversion (`core.autocrlf` or an `eol` attribute) wrote it, not a hand edit.
- **A locator one side changed** (maintainer decision of 2026-11-20, option "Auto-resolve": "pointer
  change wins, hash recomputed from the merged file and recorded in a merge revision row"; limited
  after review of the pull request to the one case where the other side's re-pin covers every line
  the new pin does). Where one side replaced a whole-file pinned locator of a site (the evidence field
  or an item's evidence clause) with a scope of the same file (a range or a symbol) and the other
  side only re-pinned the whole file, the scope wins and the whole-file locator is dropped; this holds
  in the evidence field and in an item alike. The pin of the winning locator is recomputed from the
  merged file under its scope when all the conditions above hold with the sides' attestations read per
  locator: the file at the commit that changed the locator hashes, under the new scope, to the pin it
  wrote, and the file at the other commit hashes, as a whole, to the pin that side re-pinned it to.
  The other side's whole-file pin covers every line of its file; the confinement below keeps the
  changer's own edits outside its scope out of the pin and keeps the scope where the changer had it. If the pinned file is itself unmerged the record
  waits on it as above.
  **Refused, naming why:** an old locator that is itself a range or symbol (the other side's re-pin
  covers what that scope held in its own file, whose lines can sit elsewhere than in the changer's, so
  no containment of the new scope is judged), a locator moved to another file, and a locator both
  sides dropped and replaced with different ones (naming the old locator and both new ones). A new
  locator is paired with a dropped one of the same file first; a side that changed a locator while the
  other did not re-pin it keeps its pin as written, as any one-sided change does. The merge row lists
  the pin with the changed side's hash and the old locator's re-pin as the two hashes and names the
  locator it replaced; `pins` in the report carry `moved_from` and `moved_from_pin`. A new locator
  paired only with a dropped locator of another file is refused as a move, so an unrelated swap (drop
  `a.rs`, add `b.rs` at one site) is refused too: conservative, kept.
- **A scoped pin is confined.** For every pin recomputed with a scoped attestation (the changed side's
  new scope in the case above, and either side's range or symbol when both sides re-pinned one), that
  side's file must equal the merge base outside the scope's span (the same lines before it and after
  it; a symbol's span is resolved in that side's file), or the record is refused naming the side and
  that it changed code outside what it pinned. The scope is then resolved in the merged file. A range
  names lines by number, so the lines before the merged span must equal the side's lines before its own
  span: an edit above a range by the other side refuses the record even when it moves no line (the pin
  then needs an ordinary re-pin). A symbol resolves by name, so when the other side pinned the whole
  file (the narrowing case) nothing above a symbol is held: that side attests every line and the changer
  is confined. The symbol is resolved by name in the merged file, so it can land on a different function
  of that name than the one narrowed (the other side renamed `g` to `h` and added a new `g`, say); every
  pinned line is still attested by the whole-file side. When every attestation is scoped (both sides
  re-pinned one range or symbol), the lines before and the lines after the merged span must equal the
  side's own (each side is confined only relative to its own span start, and a symbol's span can start
  earlier in one side's file, by a doc comment it pulled in, than in the other's), so every line of the merged span
  lies inside both sides' spans: a range where a side inserted or deleted lines, or two sides' changes
  that together move the span's end (two deletions inside one range, two edits that extend a
  brace-balanced symbol past both), are refused. This is a guard for honest merges, not a defence
  against a hostile branch.
  `BUG-211`, `BUG-212`.
- **A new locator that competes.** A new locator that could replace more than one dropped locator of
  the same file at its site, or shares one with another new locator on its side, is refused, because
  which pin it replaces would depend on the order the locators are written in. So splitting one
  whole-file pin into two scopes at one site is refused and needs a manual re-pin.
- **A pinned file still unmerged.** When the pinned file is itself unmerged (for example a source file
  both sides edited), its merged content does not exist yet. The refusal says so, the conflict carries
  `waits_on` (the files), no re-pin is suggested, and when the run has such a record its `next_action`
  is `git diff -- :/<file>...`, the first command a person can run, and its reason gives the rest of
  the order of work as commands: resolve the conflicts in those files, run `git add -- :/<file>...`,
  then `urzua fix --conflicts` *(decided by the builder on 2026-11-16, pending maintainer review)*.
- **Version.** The higher of the two versions (refused when either is not dot-separated numbers). When
  the rows the merge adds against a side, or the merge row, are of a class `version_bump.classes`
  names and the version would equal that side's, its last number is incremented once more, so the
  record passes `revision-log.version-bump-required` against either branch *(decided by the builder on 2026-11-16, pending maintainer review)*.
  A person's row is never rewritten. Only when the merge sets a version neither side had (the bump
  above) does it say so: the machine row states the number, that it is the next above the higher
  side's because `version_bump` requires a new version for the rows the merge combines, and, when a row
  one side added (never a row already in the base) names the version field or a bump together with a
  side's version, that this number was true on its branch and is superseded; when no pin was
  recomputed, a row opening `Version set on merge (attested by ...)` says this alone. A version taken
  from one side writes no such row *(decided by the builder on 2026-11-16,
  pending maintainer review)*.
- **The merge row.** When a pin was recomputed one row of the type's evidence class is appended (see
  below): `Pin recomputed on merge (attested by NAME, identity: SOURCE, actor: ACTOR): both merged sides
  re-pinned, ...`, naming each pin's locator and site, the two sides' hash prefixes in sorted order *(decided by the builder on 2026-11-16, pending maintainer review)* and the new one, saying the
  new hash is git's automatic merge of the two re-pinned commits, that no person wrote a reason for the
  merged state and that a reviewer should confirm it. The row's opening is the marker
  `urzua_core::conflicts::merge_row_locators` parses *(decided by the builder on 2026-11-16, pending maintainer review)*.
- **Refused and left untouched:** a conflict anywhere else, a pin failing the conditions above, a record
  with no merge-base, ours or theirs stage, a record starting with a byte-order mark, and a record git
  lists as unmerged whose working file has no conflict markers and no merge row of this command (resolved
  by hand: stage it). One whose working file carries a merge row neither side has is reported
  `resolved` ("already resolved by fix --conflicts") with exit 0 *(decided by the builder on 2026-11-16, pending maintainer review)*. A stale pin that did not conflict
  is never moved.
- **Apply** (`--conflicts --apply`): `--ids` or `--force`, never implied *(decided by the builder on 2026-11-16, pending maintainer review)*; identity resolved exactly
  as for `--apply` (including the `--actor` requirement for a `gh-verified` identity); the lock is taken
  before anything is read. Nothing is staged: `next_action` is `git add` of the resolved records, and
  the merge or rebase is continued by the user.
- **Status:** `nothing-conflicted` (git lists no unmerged file), `no-record-conflicts` (unmerged files,
  none a record), `resolvable`, `resolved`, `partial-failure` (a record refused or an `--ids` entry
  matching no unmerged record, also when git lists no unmerged file at all; exit 1), or `not-run`. So a run with nothing conflicted is told apart
  from one that resolved nothing (`SPEC-1`).
- **Suggested next action.** While git lists unmerged records, `check` and `fix --rebaseline` raise an
  `unmerged-records` warning notice and their `next_action` is `urzua fix --conflicts`.
- **Review notice.** `check --base` raises a `pins-recomputed-on-merge` warning notice listing every
  record and locator whose merge row the diff adds, so a reviewer sees each pin no person gave a reason
  for. A notice and not a finding: the row is legitimate, and the reviewer, not the gate, confirms it
  *(decided by the builder on 2026-11-16, pending maintainer review)*.
- **Residual trust.** Each side's pin is trusted as committed, even when no reason row accompanies it:
  the checks prove the file at a commit hashes to that commit's pin, not that anyone reviewed the
  change behind it. And a clean textual merge can combine two attested changes into a state neither
  side reviewed. That is why `check --base` lists every pin recomputed on merge for the reviewer.
- **Kinds to come.** The report lists conflicts by `kind` so that renumbering a record whose number a
  merge made a duplicate (`MILE-229`) slots in beside `pin`; it is not built yet.

## The evidence class of re-pin rows

A type may declare `revision_log.evidence_class` (maintainer decision of 2026-11-16, `ADR-67`
amendment). The rows of `fix --rebaseline --apply`, `reverify --apply` and `fix --conflicts --apply`
carry it in place of `tool_class`; the class must be in the type's `classes`. Left out of
`version_bump.classes`, a re-pin does not bump the version, and the row is still written. A type that
declares none writes `tool_class`, as before. Rows already in records keep their class.

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
  `changed` or `unresolved`. `drift_acknowledged` (absent when empty) lists the drifted locators of a
  category declared `drift: acknowledge` that a renewal would write into its row, as the row names them,
  so a record the policy lets through is listed as renewable with its drift shown. `refused` lists why a record cannot be written; with `--pin-only`,
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
  this command exists to prevent), where the drift policy of the locator's category is `block`, the
  default (`SPEC-3`, `drift`). A refusal for drift in a category with a pin to move names the re-pin
  remedy; one for a category outside `require_pin` (a `spec:` locator, which `--pin-only` cannot pin)
  names the category, that its policy is `block` (declared, or by default), and the `drift` setting
  that lets a reviewer acknowledge it. Under `acknowledge` the renewal is written and the `Re-verified`
  row gains a clause `; Drift acknowledged: <locator>, ...` between its `Evidence` list and the
  `Reason` (the `;` ends the evidence list, so the row parses unambiguously): a cited record as
  `<category>:<id> (latest revision <date>)`, any other file as
  `<category>:<path>@sha256:<12 hex> (last changed in <short sha>)`, the commit being the newest one
  that touched the file after the `Realized-by` line was last touched (omitted when git cannot say).
  A drifted pinned locator of an `acknowledge` category is re-pinned by the same renewal: the new
  hash is written over the stale one on the `Realized-by` line or item clause (nothing else on the
  line moves), and the clause records `<category>:<path>@sha256:<old 12 hex> -> <new 12 hex>`
  (a cited record: `<category>:<id> (latest revision <date>; pin <old> -> <new>)`), attested by the
  same `--by`, `--actor` and `--reason`; so the renewal leaves no stale pin for
  `embodiment.consistency` to report. An unpinned locator is only recorded, and the renewal's
  row names it (`category:locator`, a cited record `category:ID`) and dates the acknowledgement:
  `embodiment.consistency` judges that locator's drift from that day on, so a change of the cited file on or
  before it (day granularity: later the same day too) no longer reports once `drift_grace_days` has passed, and
  a later change does; a locator the row does not name keeps drifting. Only a row in the exact shape below
  acknowledges anything. A record whose drift all lies in `acknowledge` categories is not refused because
  `embodiment.consistency` reports it (the renewal clears it); a disagreement that remains without that
  drift still refuses. Under `block` a moved pin
  is refused as above, and `--pin-only` never moves a pin. Under `ignore` the drift is neither refused nor listed; a stated `Embodiment` that `embodiment.consistency` would report;
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
  `Re-verified (attested by NAME, identity: SOURCE, actor: ACTOR): N locators re-resolved, P hash-checked, S spec locators checked by git history only. Evidence: ...[; Drift acknowledged: ...] Reason: <reason> **Why:** the claim was <age> days old against a <days>-day period.`
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

## When a pinned file was merged by hand

`fix --conflicts` recomputes a pin only over git's automatic merge of the pinned file. When the file was merged by hand
(or edited during the merge, or changed on a side without a re-pin) it refuses the record, and its refusal and
`next_action` give the sequence that passes `diff.pin-changed-unattested`: take the merged-in side's version of the
record, whose pins are the base's (`git checkout --theirs -- :/<record>` while merging, `--ours` while rebasing; the
report's `take_side` names which), `git add` it, then `urzua fix --rebaseline --apply --ids <record> --actor agent
--reason "<what the hand-merged code changed>"`. Re-pinning from the branch's own pin instead would leave a chain that
starts from the branch's hash, not the base's, and the rule would report it. Taking the merged-in side drops that
record's own branch rows, which the new re-pin row supersedes. A path that resolves the record's text while leaving the
hand-merged pins stale (so only the re-pin is left to do) is not built (decided by the builder on 2026-11-16, pending
maintainer review).

## Path gates bind every command that writes for someone (`MILE-289`)

A path gate in the `authority` table (`gates: [{paths, by}]`) names who may change a path: the group `by`. The editor
hook refuses an agent's edit there; the commands that write records on someone's behalf now refuse the same write, through
one function, `authority::path_gate_refusal`, before anything is written (decided by the builder on 2026-11-16, pending
maintainer review):

- **`fix --apply`** (every repair), **`fix --rebaseline --apply`**, **`fix --conflicts --apply`** and
  **`reverify --apply`** take an attestation. A record a path gate covers is written only when the run is `--actor human`
  and the `gh`-verified login (the identity itself, or the login verified beside an asserted name) is in the gate's group:
  the same identity and actor attestation the revision row records. Otherwise the record is failed (for `--conflicts`,
  `refused`) with a message naming the gate, its group and its logins, and saying that one of them runs the command with
  `--actor human` under a `gh`-verified identity. `--actor agent` is always refused. `fix --pin --apply` writes through
  the same check.
- **`waive --apply`**, **`index --apply`**, **`migrate ids --apply`**, **`migrate filenames --apply`** and **`new`** take
  no actor, so they cannot say who runs them: under a path gate they refuse, naming the group, and the change is made by
  hand by one of its members. `migrate filenames` checks every file it would rewrite or rename first and writes nothing
  when any is covered, so a migration never lands half done. `migrate config` takes no `--apply` and is unchanged.

A repository with no path gate behaves exactly as before. A path given as `./x`, `a//x`, `x/` or `a/../x` is
normalised inside `path_gate_refusal`, so no caller's spelling escapes a gate.

**What this gate rests on.** `--actor human` is the caller's claim, as everywhere else (`ADR-89`), so the gate rests on
the `gh`-verified login being in the gate's group. An agent running under a maintainer's own token can pass
`--actor human` and write, and the row then records `actor: human`. The gate stops an agent with its own identity or
with no verified login, and an accidental write; it is weaker than the edit hook, which refuses the agent whatever it
claims. CI's `diff.decider-required` judges the pull request that carries the change.

## Pinning evidence with a row (`fix --pin`)

`urzua fix --pin` is the attested way to change what a record's evidence pins, so `diff.pin-changed-unattested`
(`SPEC-2`) can tell a reviewed change from a hand edit (decided by the builder on 2026-11-16, pending maintainer review).
It works on any record of any type and status; no expiry period is needed.

- `urzua fix --pin [--ids <record,...>]` lists every unpinned `code:` and `test:` locator whose file can be read, with the
  hash it would write, and prints the apply form as `next_action` (its `<...>` reason in `next_action_inputs`). A locator
  it cannot pin (a missing file, a scope that finds nothing) is listed under `not_pinned` with the reason.
- A pin is of the working tree at the time of pinning. When a pinned file has uncommitted changes the report carries a
  `pin-uncommitted` warning notice naming it, so the file is committed with the record.
- `urzua fix --pin --apply --ids <record,...> --actor agent --reason '<why this evidence carries the claim>'` pins them
  and appends a `Pinned` row naming each locator and its hash.
- `--replace OLD=NEW` (one record) moves a pin to another locator, a new scope or a renamed file: `OLD` is the pinned
  locator as `path` or `path#scope`, prefixed with its tier (any evidence category: `spec:`, `code:`, `test:`, `record:`) when the record cites it in more than one
  (a bare `OLD` that matches two tiers is refused, naming the spelling of each tier it is cited in), `NEW` the locator that replaces it (optionally `code:` or `test:`), hashed now. The
  row names the old locator and hash and the new locator and hash, in full.
- `--remove OLD` (one record), `OLD` named as for `--replace`, removes a pinned locator from the evidence, with a row naming it and its hash. When the `Realized-by` field holds the pin it edits that field's lines only, folded or not, leaves a YAML comment line alone and keeps the file's line endings; the same pin in a Done-means item stays, and the entry's `still_cited_by_items` lists each item that cites it as `{"item": <number>, "line": <clause line>, "locator": "<tier>:<locator>@sha256:<hash>"}` (the maintainer, 2026-11-25, "Yes: field copy only"). When the field holds no copy and items do, the first item's copy is removed, since it is the evidence being withdrawn, and the other items are listed the same way. `--replace` moves every copy. The lines are located in the text re-read under the fix lock. A record's
  only locator is removed by hand.

Each write takes `--reason` (held to `attestation.reason`), an identity and `--actor`, takes the fix lock, re-reads the
record and passes the path-gate check above. The finding of `diff.pin-changed-unattested` prints the command for its case,
and the scoped-locator-without-a-pin finding of `embodiment.locator-exists` suggests the listing.

## The evaluation date (`--as-of`)

`fix` and `reverify` reports carry a top-level `as_of` (`{"date", "source"}`, `SPEC-1`). In detect mode the
global `--as-of YYYY-MM-DD` makes the claim age, the expiry and the audit grace read that date, so `reverify
--as-of <date>` lists the records whose claim will have expired by then. With `--apply` (including
`fix --rebaseline --apply`) the option is a usage error (exit 2, nothing written): the revision row and the
anchor carry the real date, and a different one would falsify the history. A command that never sees a stated
date refuses the option rather than ignoring it.

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
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 04e1ba443038 -> 23e5e6b703b5. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. fix.rs: with_locator_env now takes the records, which fix passes (one added argument, reformatted), and apply and rebaseline are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.34`. The pins on this spec's `Realized-by` locators were re-baselined against the files as they are after the sharded CI and the milestone `Kind` amendment landed on main (the `Re-baselined` row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97`: every stale pin is re-pinned with its real delta before expiry is enabled. | **structural** |
> | 2026-11-08 | Bumped to `0.35`. `urzua reverify` hardening: a locator outside the repository or untracked no longer resolves for a pin; a first pin is no longer spliced into a longer anchored locator (`a.rs#Foo::bar`); any actor but human is refused where the period's level is `error`; `--force --ids` renews the named record; detect lists a record with no usable revision log as refused. Documents the four. **Why:** the round-4 review findings on `reverify` (`ADR-97`); no change to what an accepted run writes. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 23e5e6b703b5 -> 5a67695c913d, `rust/crates/urzua-core/src/fix.rs` c9cd87e7b00e -> 88c815c0639a. Reason: Read the diff of each pinned file: core fix.rs moved apply_rebaseline's evidence-line lookup and revision-row tail into shared helpers (evidence_edit_lines, append_log_rows, revision_log_refusal) that apply_reverify also calls, with the same rows and error text; cli commands/fix.rs gained validate_reason, back_pointer_evidence, into_failures and identity_notices extracted from fix and reverify with the same behaviour; core reverify.rs calls the helpers and its first-pin matcher no longer splices into a longer anchored locator; cli commands/reverify.rs now refuses a record with no usable revision log in detect, refuses any non-human actor at an error-level period, and honours --ids under --force. Each claim these records make about those files still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.36`. `urzua reverify` lists, dates and renews a milestone whose evidence is only its Done-means item clauses, pinning the item locators; the refusal for a record with no evidence names both sources. **Why:** `ADR-97` amendment of 2026-11-08. | **substantive** |
> | 2026-11-08 | Bumped to `0.37`. `fix` and `reverify` reports carry `as_of`; the global `--as-of` applies to their detect modes and is refused with `--apply`. **Why:** maintainer decision of 2026-11-08 (the second `ADR-97` amendment of that date). | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 5a67695c913d -> 751d9bdf6982. Reason: Read the diff of each re-pinned file. commands/fix.rs: the FixReport construction gains as_of; no other change. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 88c815c0639a -> aff4037c8a9a. Reason: Read the diff of each pinned file since the old pin; the claim still holds. fix.rs: the only change is that the re-baseline refusal message names the type's declared item section instead of the literal Done means (a new items_section field carried to it, with a test of the wording); how pins are found and rewritten is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.38`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` aff4037c8a9a -> db0e96b74575. Reason: fix.rs: since the previous pin only the pin-reading loop in detect_rebaselines moved into a helper (note_failed_pin) to stay under the SPEC-22 function bound; behaviour is unchanged, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 88c815c0639a -> db0e96b74575. Reason: fix.rs: this change makes the re-baseline refusal message name the type's declared item section instead of the literal Done means (a new items_section field carried to it, with a test) and moves the failed-pin listing in detect_rebaselines into a helper to stay under the SPEC-22 bound; unrelated to the claim this record evidences, which still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 751d9bdf6982 -> 841885614c38. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: fix.rs gained excused_evidence_notice, the info notice with the count of lines the ignore_marker kept out of evidence, emitted by fix; nothing else in fix changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.39`. The pin on `rust/crates/urzua-cli/src/commands/fix.rs` was re-baselined (the `Re-baselined` row above) after `urzua fix` gained an info notice counting the lines a declared back-pointer `ignore_marker` kept out of evidence; no behaviour this spec describes changes. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 841885614c38 -> ac3b642b2a4a, `rust/crates/urzua-core/src/fix.rs` db0e96b74575 -> 13699787830d. Reason: Read the diff of each re-pinned file against main. cli/commands/fix.rs: Notice literals use Notice::new or gain detail: None; no behaviour change; core/fix.rs: Notice literals gain detail: None and the unplaced-pin message names the type's declared items.section. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.40`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` ac3b642b2a4a -> 9e52be4749a2, `rust/crates/urzua-core/src/fix.rs` 13699787830d -> bfd53b872aaa. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: commands/fix.rs: split run into run_checked, check_usage, select_scope, detect_repairs_in, apply_repairs, write_repair and authority_refusal, and run_rebaseline into rebaseline_checked, apply_rebaselines and write_rebaseline, with RebaselineArgs and ReportParts replacing two #[allow(clippy::too_many_arguments)] parameter lists; main's #444 excused-evidence notice (excused_evidence_notice, with_excused_notice applied to the unevidenced notices in detect_repairs_in) and #445 detail: None are carried in the split code; core fix.rs: split detect_repairs into the RepairScan methods embodiment and status, and detect_rebaselines into stale_pins and stale_item_lines; main's #445 detail: None on the two unevidenced notices is carried. Behaviour unchanged, outputs byte-identical (check, check --base origin/main, fix, fix --rebaseline, reverify, reverify --as-of 2027-06-01 and --help stdout, stderr and exit compared against a binary built from origin/main). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.41`. The pins this spec's `Realized-by` holds on files split under the SPEC-22 function bound were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.42`. The `Why` of a `Re-baselined` row names who re-verified by the actor kind (an agent, a person, or the attesting actor) instead of always saying a reviewer did. Earlier rows are unchanged. **Why:** an agent's re-pin read as if a reviewer had already re-verified the claim, which it had not; the row now says what the attestation records. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` bfd53b872aaa -> 98b3bebce0fd. Reason: fix.rs: read the diff since the previous pin; it adds Attestation::rebaseline_why and uses it for the Why text of the re-baseline revision row (worded by actor kind) plus its test; nothing this record's claim covers changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 98b3bebce0fd -> 0b23d692333e. Reason: Read the diff of fix.rs since the old pin against what this record evidences: the only change is log_row, which now writes a pipe in a revision-row narrative as an escaped pipe so the row keeps its columns (plus its unit test); the behaviour this record evidences is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.43`. The pinned `fix.rs` evidence moved when `log_row` began escaping a pipe in a revision-row narrative; no spec text changed. **Why:** the repository checker asks a dated structural revision row, such as the re-pin above, to carry a version bump. | **structural** |
> | 2026-11-09 | Bumped to `0.44`. Documents the per-category drift policy of `reverify` (`block`, `acknowledge`, `ignore`): the corrected refusal for a drifted locator outside `require_pin`, `drift_acknowledged` in the detect report and the `; Drift acknowledged` clause of the `Re-verified` row (a non-record file named with the commit that last changed it; a drifted pinned locator of an `acknowledge` category re-pinned by the same renewal, old -> new hash recorded). **Why:** `ADR-97` amendment of 2026-11-09 (drift policy); the refusal told the reader to run `--pin-only`, which cannot pin a `spec:` locator, and a renewal under `acknowledge` left the pin of a moved code or test locator stale. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 0b23d692333e -> b5bfb2aa1386. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. rust/crates/urzua-core/src/fix.rs: replace_exact_pin becomes pub(crate) so reverify can reuse it; its body and every caller are unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.45`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Bumped to `0.46`. `reverify` under `drift: acknowledge` does not refuse a record whose only drift is in acknowledging categories because `embodiment.consistency` reports it past `drift_grace_days`, and the acknowledging `Re-verified` row dates the acknowledgement for unpinned locators. **Why:** `ADR-97` drift-policy amendment, consistency fix: `acknowledge` worked only inside the grace window. | **substantive** |
> | 2026-11-09 | Bumped to `0.47`. The acknowledging `Re-verified` row names each acknowledged locator and clears only those, the date has day granularity, and only a row in the tool's exact shape acknowledges anything. **Why:** `ADR-97` item 6: a row cleared every unpinned locator of its category, and any text containing the words cleared them. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 9e52be4749a2 -> 5d6f30edea8f. Reason: merge of origin/main into the MILE-207 branch: the pinned files differ from main's pins only by this branch's path-scoped source read (scoped sources in discovery.rs, check.rs, back_pointer.rs, runner.rs, table.rs, report.rs, fix.rs) and the 'not judged' notice wording; each record's own claim is untouched **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-14 | Bumped to `0.49`. `fix --apply`, `fix --rebaseline --apply` and `reverify --apply` require `--actor agent` or `--actor human` when the attestation identity is `gh-verified`; without it the run is `not-run` and writes nothing, so a row can no longer record `actor: unspecified` under a verified login. **Why:** the external review found 208 re-pin rows attributing an agent's re-verification to the maintainer; `ADR-101` (people own decisions). | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 5d6f30edea8f -> 8108f0b26a58, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 27c91b75ad61 -> 50ac84d1005f. Reason: commands/fix.rs: resolve_attestation now refuses an apply with a gh-verified identity and no --actor; the detect, repair and re-pin logic this record cites is unchanged; fix_rebaseline_integration.rs: the gh-wins test now passes --actor agent and expects actor: agent; the rest is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.50`. `fix` (Tier 1 `Embodiment` and the `Status` repair) counts only the `Realized-by` locators that resolve (the test `embodiment.locator-exists` applies: tracked, present, a scope that resolves): a locator naming a file that is not there is no evidence, so it neither lifts a computed tier nor is written as `Verified`, `Implemented` or `Done`; a repair that lowers a claim names the locators it did not count in its `evidence`. `fix --rebaseline --apply` refuses a `--reason` of fewer than 3 words or 12 characters. **Why:** an external review found `fix --apply` writing `Verified` from a locator to a file that did not exist. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 8108f0b26a58 -> 8e0c4f850c4d, `rust/crates/urzua-core/src/fix.rs` b5bfb2aa1386 -> 4a03c290767b, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 50ac84d1005f -> 29408f8d9f3c. Reason: Read the diff of each re-pinned file against main: cli fix.rs refuses a rebaseline --reason under 3 words or 12 characters; fix_rebaseline_integration.rs passes sentence reasons and adds the short-reason test; core fix.rs counts only resolving locators for Embodiment and Status and puts header-less types out of rebaseline scope. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.51`. `fix --rebaseline` runs with neither `embodiment.consistency` nor `status.delivery-agreement` declared; the reason minimum is the configuration's `attestation.reason` (defaults 3 words, 12 characters) and applies to `reverify --apply` too. **Why:** a review found the re-pin `embodiment.pin-stale` suggests refused in a repository that declares neither repair rule, and the ADR-101 amendment of 2026-11-15 puts behaviour in configuration. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 8e0c4f850c4d -> 8de563649d5c, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 29408f8d9f3c -> e20101f904c9. Reason: Read the diff of each re-pinned file since its previous pin: cli fix.rs runs --rebaseline before the repair-rule guard and reads the reason minimum from attestation.reason instead of two constants; fix_rebaseline_integration.rs adds the attestation.reason test. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.52`. Tier 2 ships for one case: the missing inverse of a declared relation, listed and written by `fix`, never changing a `Status` or overwriting a different value. **Why:** an agent could make a paired change to two records only through the shell (`MILE-287`). | **substantive** |
> | 2026-11-16 | Bumped to `0.52`. `fix --conflicts` lists the records a merge or rebase left unmerged by kind of conflict and, with `--apply`, resolves kind `pin` (sides differing only in evidence pins, the version and appended revision-log rows), recomputing a pin only when the merged file is git's automatic merge of two commits whose files hash to their sides' pins, with a machine-written row; any other conflict is refused and left untouched; `check` and `fix --rebaseline` suggest it while records are unmerged and `check --base` lists recomputed pins; `revision_log.evidence_class` is the class of re-pin rows. **Why:** `MILE-284`; maintainer decisions of 2026-11-16 recorded in the `ADR-67` amendment; a review found a hand edit during the merge and an unpinned change on the default branch pinned without a reason. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 8de563649d5c -> 73f00d578860, `rust/crates/urzua-core/src/fix.rs` 4a03c290767b -> 958a755d87a6. Reason: cli fix.rs adds the missing-inverse repairs to the detect pass, lets fix run when only a reciprocity rule is on, and keeps a run that found an inverse repair from reading as not-run; the Embodiment, Status and rebaseline paths are unchanged; core fix.rs gains the inverse field and InverseOf type on Repair, splits apply_repair into replace-in-place and a front-matter insert for an absent inverse field, and words the revision row of an inverse repair; the Embodiment and Status repairs and their rows are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 8de563649d5c -> f81c730e2417. Reason: commands/fix.rs dispatches --conflicts to its own module, refuses it beside --rebaseline, --stdin-path, --reason or PATHS, and adds the unmerged-records notice to the rebaseline listing; repair and re-pin logic is unchanged. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 4a03c290767b -> a2bf44749480, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` e20101f904c9 -> 9a053f2ed41d. Reason: core fix.rs: apply_rebaseline writes its row with the type's evidence class (tool_class unless evidence_class is declared) and a test shape literal gains evidence_class; the detect and re-pin logic is unchanged; fix_rebaseline_integration.rs adds a test that a declared evidence_class is the class of the re-pin row; existing tests are unchanged. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-cli/src/commands/fix.rs` 73f00d578860 and f81c730e2417 -> 913bdc4281bf, `rust/crates/urzua-core/src/fix.rs` 958a755d87a6 and a2bf44749480 -> ad8613947560; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **structural** |
> | 2026-11-16 | Bumped to `0.54`. The inverse repair keeps the field's written form (flow list, block list, quoted or plain scalar) and does not write a protected path; the `--tier` refusal names the Tier 2 repair that runs without the flag. **Why:** review of pull request 546: a bracketed or block list failed to apply, and the repair would have written a file the editor hook refuses an agent. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 913bdc4281bf -> 8453a75ad47b, `rust/crates/urzua-core/src/fix.rs` ad8613947560 -> 88bdeae08fa5. Reason: cli fix.rs: the --tier refusal now says the one tier-2 repair runs without the flag and tier 3 is not built; nothing else changed; core fix.rs: the inverse repair records the ids it adds and appends them in the field's written form (flow list, block list, quoted or plain scalar) through the new append_ids; the Embodiment and Status repairs and their rows are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 8453a75ad47b -> 90200653f67b, `rust/crates/urzua-core/src/fix.rs` 88bdeae08fa5 -> 7ea3d709cad8. Reason: cli fix.rs reports records whose pinned file changed in a warning notice and fails an --ids entry naming one with the re-pin as the reason, computing which pins are actually stale and holding back the drift repair only for those records; core fix.rs no longer writes the drift state over a stale pin, lists those records for a notice, and appends a tool row to a revision-log table that has no row yet; the drift repair itself is no longer skipped in core, and the pinned-drift list now needs a pin that no longer hashes to its file; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.55`. `fix` does not write the drift state over a stale pin and names the re-pin listing instead; a revision-log table with no row takes the first tool row (`fix --apply`, `fix --rebaseline --apply`, `reverify --apply`). Decided by the builder on 2026-11-16, pending maintainer review. **Why:** `MILE-266` and `MILE-267` -- a drift repair turned a changed pinned file green without a re-pin, and a record from a built-in template could not take a tool row. | **substantive** |
> | 2026-11-16 | Bumped to `0.56`. Re-pinned the evidence `MILE-233` changed (the rule vocabulary read from configuration); what this spec states is unchanged. **Why:** a re-pinned versioned spec carries a version above main's. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 7ea3d709cad8 -> 6b93e753f499. Reason: fix.rs: a record of a header_shape none type is counted with the new no-header outcome, still out of scope; re-baselining logic unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 7ea3d709cad8 -> 82fdfdfb9176. Reason: fix.rs only gains rejected_values and waiver_fields in a test fixture's RecordTypeConfig literal. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-core/src/fix.rs` 6b93e753f499 and 82fdfdfb9176 -> cf5aee142c59; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` cf5aee142c59 -> fcc0a952e83d. Reason: fix.rs: a record of a header_shape none type is counted with its type's header_out_of_scope, still out of scope; re-baselining logic unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.56`. `fix --conflicts` treats a pin in a Done-means item's evidence clause like one in the evidence field (items identical apart from hashes when one moved), keeps rows with the merged-in side first and this branch's last within a date in a merge and a rebase alike, resolves a later stop of a multi-commit rebase, checks index stages 2 and 3 against the two commits, refuses a symlink or mode change in the pinned file, names line-ending conversion and a pinned file still unmerged (with the order of work as `next_action`), states a version a kept row no longer tells truly in the machine row, and says the residual trust plainly. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** in real use over six pull requests the command refused about 60 percent of conflicted records, nearly all for a pin inside a Done-means item; a security re-check found the stages unchecked. | **substantive** |
> | 2026-11-16 | Bumped to `0.58`. `fix --conflicts`: a locator repeated in one site is matched and restored by its position; stage 1 is checked against the merge bases; the version row is written only when the merge set a version neither side had, reads only rows a side added, and says nothing a reader could find false; a pinned file still unmerged gives `git diff` as the next action with the rest of the order in its reason; an item clause continued onto a second line is stated as not handled. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** independent review of pull request 552. | **substantive** |
> | 2026-11-16 | Version set on merge (attested by beauwilliams, identity: gh-verified, actor: agent): `Version` is now 0.57; an earlier row's version 0.56 was written before this merge and is superseded by it. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a person's row is never rewritten. | **structural** |
> | 2026-11-16 | Version set on merge (attested by beauwilliams, identity: gh-verified, actor: agent): `Version` is set to 0.59 by this merge, the next number above the higher side's 0.58, because `version_bump` requires a new version for the rows this merge combines. A row added on a branch states version 0.58 and 0.57, which was true there and is superseded by this merge. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a person's row is never rewritten. | **structural** |
> | 2026-11-16 | Bumped to `0.60`. Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` fcc0a952e83d -> af5e5c811c17. Reason: fix.rs: the only change on this branch is one added `hooks: Default::default()` field in a Config struct literal, needed by the new optional hooks key, merged with main's own change to the file; behaviour this record cites is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Version set on merge (attested by beauwilliams, identity: gh-verified, actor: agent): `Version` is set to 0.61 by this merge, the next number above the higher side's 0.60, because `version_bump` requires a new version for the rows this merge combines. Rows added on the branches state versions 0.60 and 0.59, which were true there and are superseded by this merge. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a person's row is never rewritten. | **evidence** |
> | 2026-11-16 | Bumped to `0.62`. Adds "When a pinned file was merged by hand" (the take-side sequence, `take_side` in the report); `fix --pin` lists `not_pinned` locators and warns on an uncommitted pinned file. Adds "Pinning evidence with a row (`fix --pin`)": a first pin, `--replace OLD=NEW` and `--remove OLD` on any record, each with a reason and a row `diff.pin-changed-unattested` reads; path-gate paths are normalised; "What this gate rests on" states that `--actor human` is a claim and the gate rests on the verified login. Adds "Path gates bind every command that writes for someone": `fix --apply`, `fix --rebaseline --apply`, `fix --conflicts --apply` and `reverify --apply` write a path-gated record only for `--actor human` under a `gh`-verified login in the gate's group; `waive`, `index`, `migrate ids`, `migrate filenames` and `new` refuse under a path gate; the refusal names who can make the change. A re-pin row is what `diff.pin-changed-unattested` (`SPEC-2`) reads. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** `MILE-289` -- `fix --apply --actor agent` rewrote a path-gated record the edit hook would have refused. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 90200653f67b -> 317aab0052ff, `rust/crates/urzua-core/src/fix.rs` af5e5c811c17 -> f95de51150cf, `rust/crates/urzua-cli/tests/fix_paths_integration.rs` 73cb06a1782f -> efefbffa3dc1, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 9a053f2ed41d -> a3485ebbfc5e. Reason: cli fix.rs routes --pin, --replace and --remove to the new fix --pin command and refuses a path-gated write unless a verified member runs it as a human; core fix.rs adds the fix::pin module and moves a misplaced doc comment back onto apply_rebaseline; fix_paths_integration.rs has the path-gate tests for fix --apply; fix_rebaseline_integration.rs adds tests for the pin rule: a renamed record, a hand-added first pin, a moved pin, a removed pin, the base level, and fix --pin; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` a3485ebbfc5e -> e99a142c76d0. Reason: fix_rebaseline_integration.rs adds tests for rename pairing, reordered and inserted citations, the swaps notice, a moved config and fix --pin's listing; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Bumped to `0.63`. `fix --rebaseline --apply` and `fix --pin --apply` create the revision log, in the type's template's or declared shape, on a record that has none, with the evidence row as its first row; detect marks it `creates_revision_log`; a record with no body section is refused with a `next_action` that adds a heading. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** `MILE-310` -- in an adopting repository nearly every first re-pin met a record with no log and the log was written by hand, the edit `diff.pin-changed-unattested` exists to tell apart from a tool-written row. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 317aab0052ff -> e80f15fd68a9, `rust/crates/urzua-core/src/fix.rs` f95de51150cf -> 0b2c8b793c0f, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` e99a142c76d0 -> 8fde6c3306f1. Reason: cli fix.rs creates the revision log for a re-pin on a record with none, marks it creates_revision_log, and refuses a record with no body section with a next_action; core fix.rs declares the new_log module and adds a creates_revision_log field to Rebaseline, with apply_rebaseline itself unchanged; fix_rebaseline_integration.rs adds tests for a created log on re-pin and first pin and for the refused bodiless record; the claim this record makes still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Bumped to `0.64`. A created log is placed by the template's section order; a record ending inside an open fence, or whose committed copy had a log, is refused in detect and apply; the no-body `next_action` runs as printed; `fix --pin` lists `creates_revision_log` and refuses as `--rebaseline` does; the narrative header cell is `revision_log.narrative_column`. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** review of `MILE-310`'s pull request -- a hand-deleted log could be replaced by a tool row and pass `check --base`, and a log appended last broke `section.order`. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` e80f15fd68a9 -> 372845c7b4a6, `rust/crates/urzua-core/src/fix.rs` 0b2c8b793c0f -> a2c71a99c6e3, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 8fde6c3306f1 -> 00d2df20a22b. Reason: cli fix.rs refuses to create a revision log when a committed copy had one, places a created log by the template order, and makes the heading next action run as printed; core fix.rs adds narrative_column to a test helper shape, with apply_rebaseline and the behaviour cited unchanged; fix_rebaseline_integration.rs adds tests for the restored-log refusal, template placement, the open fence, narrative_column and fix --pin listing; the claim this record makes still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Bumped to `0.65`. Before creating a revision log, `fix --rebaseline` and `fix --pin` read the record's copy at `HEAD` and at the merge base with the base ref (`--base`, default `origin/HEAD`; the base ref's own copy when no merge base is visible), following a detected rename or the record's `Stable-Id`, and refuse when either copy has a log. With no base ref resolving this falls back to the record's recent history and is best effort, said by a `fix.revision-log-base` warning. A log deleted by hand can still pass `check --base` when the record is not re-pinned; the durable guard is a diff rule reporting revision-log rows removed between base and head, tracked separately. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** review of `MILE-310`'s pull request -- a log deleted on a branch behind a rename, a re-add, many commits or a shallow clone could be replaced by a tool-written one. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 372845c7b4a6 -> 1c64ab73b135, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 00d2df20a22b -> e949db2be36d. Reason: cli fix.rs moves the revision-log creation helpers to a new evidence_log module, threads a shared LogHistory and the new --base option through the re-pin path, and adds its notice; fix_rebaseline_integration.rs adds tests refusing a log deleted behind marker toggles, a rename, a re-add by Stable-Id, a shallow clone and the earlier cases against --base main; the claim this record makes still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-16 | Bumped to `0.66`. The record's recent history is read even when a base ref resolves, so a base at or after the deletion (`--base HEAD`, the branch itself) no longer lets a log be replaced; a run that plans or creates a log reports `revision_log_base` and warns when the base is not remote-tracking or its merge base is `HEAD`; the created log's row names its base. In a batch the first refusal's command is the report's `next_action`. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** review of `MILE-310`'s pull request -- an agent-chosen base silently disabled the restore check. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 1c64ab73b135 -> a24bc3d1b00e, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` e949db2be36d -> e34ae347e45e. Reason: cli fix.rs reports revision_log_base when a re-pin plans or creates a log and names the base in the created log row; fix_rebaseline_integration.rs adds tests that a base at or after the deletion is refused and that a created log records its base in the report and the row; the claim this record makes still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` a2c71a99c6e3 -> 9cc05b2200bb. Reason: Merged main into the MILE-259 target-type branch. config.rs: a pointer_fields entry may be {name, targets}, Config gains a skipped pointer_targets map, RelationPair gains optional target_types, new load error InvalidPointerTargets; every other key unchanged. rules: new rule pointer.target-type (catalog.rs, pointer.rs, table.rs; default-on at error, not-enabled when nothing is declared); relation.rs cross-type-disallowed defers to it. graph.rs lists relation-pair edges. fix.rs, migrate.rs, property.rs, type_scope.rs, sequence.rs: fixtures add the new fields; fix.rs also carries main's change that a re-pin creates a missing revision log. check_integration.rs: the header-none fixture declares a target list. SPEC-3 documents the keys. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` a2c71a99c6e3 -> e67d97a794d2. Reason: fix.rs: a test fixture's RecordTypeConfig literal gains field_values: None; no behaviour changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-core/src/fix.rs` (Realized-by) 9cc05b2200bb and e67d97a794d2 -> 5677e6488e07; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` a24bc3d1b00e -> 4b3453510163. Reason: rust/crates/urzua-cli/src/commands/fix.rs: fix reads a rule on by declaration as enabled, so it repairs a checklist status when the rule is not named; the repair itself is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 5677e6488e07 -> 75776e5768fb. Reason: rust/crates/urzua-core/src/fix.rs: a test fixture that builds a Config gains the new declared_status_entries field; no production code changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 75776e5768fb -> 931921fb10f9. Reason: fix.rs gains append_revision_row, a public wrapper that appends one tool-authored row to a record's revision log; apply, rebaseline and the row layout are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` 75776e5768fb -> d9398b0c5717. Reason: fix.rs: test and fixture Config literals gain the part_of and export fields; no production behavior changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-18 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-core/src/fix.rs` (Realized-by) 931921fb10f9 and d9398b0c5717 -> ef63c51c34e0; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **evidence** |
> | 2026-11-19 | Bumped to `0.67`. An `--ids` entry written `./`-prefixed, absolute inside the repository or with the OS separator names the same record as its repo-relative path in every mode that takes `--ids`, and a failed `--ids` entry names the dry run that lists the pending repairs as `next_action`; the exit codes of each status are stated. **Why:** a repair the editor hook named, run with an entry in another path form, wrote nothing and left no next step (`BUG-207`, `BUG-208`). | **substantive** |
> | 2026-11-19 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 4b3453510163 -> 07c6e30fd14e. Reason: Read the diff of fix.rs since the old pin and the claim still holds. fix.rs adds repo_relative_ids, which normalises each --ids entry (./ prefix, absolute path inside the repository, OS separator) to the repo-relative path before matching, applied once after the session opens, and adds unmatched_action and pending_action, which set next_action to the dry-run urzua fix (urzua fix --rebaseline for a re-pin) when an --ids entry matched nothing; no repair, drift or write logic changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-19 | Bumped to `0.68`. `reverify --ids` takes the same path forms as `fix --ids`, and an unmatched `--conflicts` entry names `urzua fix --conflicts` as `next_action`. **Why:** review of the pull request found `reverify` and `--conflicts` left out (`BUG-207`, `BUG-208`). | **substantive** |
> | 2026-11-19 | Bumped to `0.69`. A `--conflicts --ids` entry that matches no unmerged record is `partial-failure` (exit 1, `next_action` `urzua fix --conflicts`) when git lists no unmerged file too; `nothing-conflicted` is only a run with no entry that failed. **Why:** it reported `nothing-conflicted` and exited 0, a silent no-op (`BUG-209`). | **substantive** |
> | 2026-11-19 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/fix.rs` ef63c51c34e0 -> c8c3ad750b4f. Reason: fix.rs skips the Embodiment and Status repairs and the unevidenced-success notice for a record whose type the rule's types: does not list **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-20 | Bumped to `0.70`. `--conflicts` resolves a locator one side changed while the other only re-pinned the old one: the changed locator wins, in the evidence field and in a Done-means item's clause, its pin is recomputed from the merged file under its scope with each side attested by its own locator, and a locator both sides replaced differently is refused naming both; the items comparison no longer counts a pinned locator's spelling. Maintainer decision of 2026-11-20, option "Auto-resolve (Recommended)": "Amend SPEC-8/ADR-67: pointer change wins, hash recomputed from the merged file and recorded in a merge revision row." **Why:** `BUG-210`: 19 of 24 conflicted records on the merge of main into a pull request were refused for a narrowed locator, which `MILE-284` meant the command to resolve. | **substantive** |
> | 2026-11-20 | Bumped to `0.71`. The locator-change resolution is limited to a narrowing within one file (the old scope contains the new); a widening, a move to another file and a locator both sides dropped and replaced differently are refused with the reason, a new locator is paired with a dropped one of the same file, and each side is attested by its own locator (review of the pull request found a pairing across files and a recomputed pin over code the other side never re-pinned). States that a range pin is recomputed over the same lines of the merged file. **Why:** `BUG-210`. | **substantive** |
> | 2026-11-20 | Bumped to `0.72`. The locator-change resolution is limited to a whole file narrowed to a scope of itself; an old locator that is a range or symbol is refused, and the containment computation is dropped. **Why:** `BUG-210`: a second review showed the other side's re-pin of a range or symbol attests that scope in its own file, whose lines can differ from the changer's, so a contained new scope could be pinned over code that side never re-pinned. | **substantive** |
> | 2026-11-20 | Bumped to `0.73`. A recomputed pin with a scoped attestation is confined: the side's file must equal the base outside the scope's span and, for a range, the merged file must keep the side's lines before it, else the record is refused; this covers the locator-change path and the ordinary path where both sides re-pinned one range or symbol. States that the fallback pairing refuses an unrelated swap, and removes the claims that the recomputed pin covers only code a side read and that a range is recomputed over other code after a merge inserts lines above it (it is refused now). **Why:** `BUG-210`, `BUG-211`: a third review reproduced a pin over lines no side attested, via a side's edit outside its scope and a merge that shifted the range; tightening only, under the maintainer's auto-resolve choice of this date. | **substantive** |
> | 2026-11-20 | Bumped to `0.74`. The confinement also resolves the scope in the merged file: the lines before the merged span must equal the side's, and, when every attestation is scoped, the lines after it too; a new locator that competes for more than one dropped locator of its file is refused. Replaces the sentences that said the confinement keeps other code out of the span and that a symbol follows its symbol. **Why:** `BUG-211`: a fourth review reproduced a merged range or symbol span that took in a line neither side pinned (two deletions inside one range; two edits that grow a brace-balanced symbol) and a pairing that depended on the order locators were written in. | **substantive** |
> | 2026-11-20 | Bumped to `0.75`. When the other side pinned the whole file, the lines above a symbol are no longer held (an edit above it no longer refuses the narrowing); they are still held for a range and when both sides re-pinned a scope, and the refusals say the other side edited above the scope, or that the merged span does not end where the side's does, instead of saying the merge moved it. States that splitting one whole-file pin into two scopes at one site is refused. **Why:** `BUG-212`: the most common narrowing merge (a whole file narrowed to a symbol while the other side edited the file) was refused for an edit above the symbol, which the whole-file re-pin and the symbol's resolution by name make harmless. | **substantive** |
> | 2026-11-20 | Bumped to `0.76`. States that a narrowed symbol resolves by name in the merged file and can land on another function of that name; the lines before the merged span are no longer compared when both sides re-pinned a scope (each side is confined, so they are the base's on both), which changes no outcome. **Why:** review of the pull request for `BUG-212`. | **structural** |
> | 2026-11-20 | Bumped to `0.77`. Corrects `0.76`: the lines before the merged span are still compared when both sides re-pinned a scope, so removing that comparison would have changed an outcome (a symbol one side pulled a doc comment into is refused); each side is confined only relative to its own span start. The refusal for lines above a range or a both-sides symbol names what happened per scope kind. **Why:** review of the pull request for `BUG-212`. | **structural** |
> | 2026-11-21 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` e34ae347e45e -> 741d74b614d4. Reason: BUG-213: a test is appended, a_prefix_lines_record_deleted_and_re_added_elsewhere_is_paired_by_its_stable_id; existing tests are unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-21 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 741d74b614d4 -> c566717e9a27. Reason: BUG-213: the appended prefix-lines Stable-Id pairing test gained a comment explaining its long base body; existing tests are unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-22 | Bumped to `0.78`. A re-pin keeps a scope's member kind suffix (`Type::name[fn]`) as written, and a scope naming a kind no member entry declares is refused like any unresolved scope. **Why:** `MILE-352` / `ADR-74` amendment of 2026-11-22, chosen by the maintainer (2026-11-22, AskUserQuestion answer to the orchestrating agent): question "MILE-352: ... How should a locator tell them apart? Any grammar change is an ADR-74 amendment you'd accept.", answer "Declared kind suffix (Recommended)". | **substantive** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/fix.rs` 07c6e30fd14e -> e7577259ef4e, `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` c566717e9a27 -> ac8cb61dec86. Reason: Read the diff of each file against origin/main: fix.rs passes the pending-listing remedy as words instead of a string; the json helper checks every report agrees with its argv and the heading action is asserted as an sh -c command. The cited behavior is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-24 | Bumped to `0.79`. Corrects two statements to match the code: the no-body `next_action` is an `sh -c` command, not a `printf`, and the `--pin --apply` form shows its `--reason` placeholder single-quoted, as `next_action` prints it. No behaviour changes. **Why:** review of the pull request for `MILE-367`, which builds every `next_action` from a list of words. | **structural** |
> | 2026-11-24 | Bumped to `0.80`. `fix --pin --replace OLD=NEW` and `--remove OLD` accept `OLD` with its tier (`code:X`, `test:X`); a bare `OLD` that matches citations in two tiers is refused naming both, instead of a false "pinned at more than one hash". **Why:** `BUG-230`, the `pin-removed` finding for a dropped `test:` copy beside its `code:` copy named a command that refused to run. | **substantive** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` ac8cb61dec86 -> 099021e54fce. Reason: Read the diff of fix_rebaseline_integration.rs: it gains one test, fix_pin_removes_one_tier_of_a_file_cited_in_two_observed_failing, for --remove naming a tier; the existing tests are unchanged and the spec's claims about them still hold. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-24 | Bumped to `0.81`. `OLD` may carry any evidence category, and the refusal of a bare `OLD` names the tiers it is cited in; `--remove` removes the `Realized-by` field's copy of a pin and leaves a Done-means item's copy, `--replace` moves both. **Why:** `BUG-231`, the refusal offered `code:` and `test:` for a file cited as `spec:` and `record:`, and `BUG-232`, `--remove` edited the item's copy and left the field's. | **substantive** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 099021e54fce -> 5915dc15c4bb. Reason: Read the diff of fix_rebaseline_integration.rs: it gains three tests and helpers for the refusal spellings and for --remove and --replace with a pin in a Done-means item; existing tests are unchanged and the spec's claims about them still hold. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` 5915dc15c4bb -> e6f24e55efab. Reason: Read the diff since the last re-pin: cargo fmt reflowed the three new tests; no logic changed, and the spec's claims about them hold. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-25 | Bumped to `0.82`. `--remove` edits the evidence field's lines only, reads a folded field, reports the Done-means items that still cite the pin and refuses a pin cited only in items. **Why:** the maintainer's decision of 2026-11-25, given in the orchestrating agent's session and not recorded in the repository ("Yes: field copy only"); `BUG-232`, `BUG-233`. | **substantive** |
> | 2026-11-25 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` e6f24e55efab -> e939866d9091. Reason: Read the diff of fix_rebaseline_integration.rs since the last re-pin: one test is renamed and three tests are added for a folded field, a pin cited only in items and the other comma shape; the existing tests are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-25 | Bumped to `0.83`. `--remove` of a pin the field does not hold removes the first Done-means item's copy and lists the others, as before, instead of refusing; `still_cited_by_items` is a list of `{item, line, locator}`; a comment line in the field is not edited and CRLF is kept. **Why:** the maintainer's decision of 2026-11-25 ("Yes: field copy only") covered a pin in the field and an item, and refusing an item-only pin left the `pin-removed` finding naming a command that refused (`BUG-232`, the same class as `BUG-230`). | **substantive** |
> | 2026-11-25 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` e939866d9091 -> 9b0012ff465d. Reason: Read the diff of fix_rebaseline_integration.rs since the last re-pin: the shared helper pin_in_field_and_item changed, two tests' arguments changed with it, the refusal test is replaced by item-only, comment and CRLF tests, and the folded-field test gains a check --base assertion; the other tests are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
