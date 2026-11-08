---
Stable-Id: 01M6R3NK5GM1RYRRX2XVMZ0ZAK
Version: '0.129'
Date: 2026-08-20
Status: Shipped
Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs@sha256:c819006495f23fd9742941942eea3a7a4894f9eec67d3c3cfbf18561ede31f6b, code:rust/crates/urzua-core/src/rules.rs@sha256:355dd5bf590d2020ab967cb30501aa6ce610186c6631f22aa828831a6fbac0d4, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:8538aedb749292d7641c037df67b6e667d5087b0c8dca95682fef065a8429a00, test:rust/crates/urzua-cli/tests/fail_on_integration.rs@sha256:4b1707f3b18aed7b018f8b80c55f1e2091ae19148f3de6c95435a8f5fee4cab7
Embodiment: Verified
Author: beauwilliams
Subject: '`urzua check` -- discovery, rule execution, and the JSON output contract.'
Implements: RFC-1, RFC-3, RFC-10, RFC-11
Parent: SPEC-1
---
# SPEC-2 — `urzua check`

## Purpose

The validator, and the whole of Phase 0. Everything else in the roadmap parses and validates
records first, so this is the one piece nothing routes around. Cross-cutting rules — no-silent-
no-op, the permanent content-scope ceiling, the three-corpus acceptance bar — are stated in
`Parent`'s target, `SPEC-1`, and are not restated here except where this command narrows them.

`check` reads records, applies rules, and reports findings. It never writes. `--fix` is deferred to
RFC-8 and is out of scope here for a recorded reason: a bulk cross-reference rewrite caused a
documented data-loss incident in one source implementation, and repair is not safe until a revision
log makes it reversible.

## Success criteria

Done when all five hold:

| # | Criterion | Phase |
|---|---|---|
| 1 | `make check` runs `urzua check` in this repo's CI and fails the build on a deliberately broken record | A |
| 2 | It reports files examined, records read by any rule, rules executed, **and each rule's input population with its unit** | A |
| 3 | It exits non-zero when given paths that match zero files | A |
| 4 | Every rule family in §Rules is implemented and configurable | B |
| 5 | It runs green in one adopter's codebase with that adopter's hand-written linter **deleted** | C |
| 6 | It reproduces every documented bug in SPEC-1's acceptance suite against real corpora (SPEC-4) | D |

Criterion 6 belongs to the Phase 1 exit (`SPEC-1`, Exit bar), not to this spec's done.

Criterion 5 is the bar for the *product*. Criterion 1 is the bar for *momentum*: until the tool
validates this repository, every rule after it is written against corpora nobody has run.

A validator running alongside the linter it was meant to replace has failed, whatever its coverage
says.

## Phases

The command specified below is the finished shape. It is not the first thing to build, and the
ordering here exists to answer one question: **what is the shortest path to the tool running against
this repository's own records?**

Everything changes once that loop closes. Until it does, every rule is a claim about corpora nobody
has run; after it, each rule is exercised on real records the day it lands, and the project's own
governance is validated by the thing it is building.

**Corpus zero is `docs/` in this repository.** SPEC-4's anonymized corpora are required by
criterion 3 — reproducing the documented historical bugs — and are *not* required to close the loop.
That decoupling is deliberate: anonymization carries an unresolved question about whether the
corpora can be published at all, and the fastest deliverable must not sit behind the slowest one.

### Phase A — the loop closes

**Done when `urzua check` runs in this repository's CI and fails on a deliberately broken record.**
Nothing more. Unscoped: a path argument narrows which findings are reported (`BUG-67`) and which records' git history `embodiment.consistency` reads, so a one-record check does not pay for the whole corpus's blame; and this
repository's corpus is no longer confined to `docs/` -- the claim files a rule reads live beside it.

Scope:

- Git-tracked discovery, with `scope.source` reported.
- Header parsing for the record types in `docs/`.
- A minimal `.urzua/config.yaml` — record types and directories only. Not the full configuration surface,
  but a real file rather than a hardcoded layout, so the config thesis is exercised from day one
  rather than retrofitted.
- **Exactly two rules**, chosen because both fire on this corpus today (see below).
- The output contract in full: `status`, `filesExamined`, `rulesExecuted`, `scope`, `blocking`.
  The contract is cheap now and expensive to change once anything consumes it.
- Exit codes 0/1/2.
- A planted-violation test per rule.

Explicitly not in Phase A: the remaining rule families, severities, roles, boundaries, `doctor`,
the acceptance corpora.

**The two seed rules, and why these two.** Both are chosen so that the first honest run produces
findings rather than a green light — a validator whose first output is `ok` has told you nothing
about itself.

1. **Header format consistency within a record type.** At the time Phase A was scoped, this
   corpus's 22 records carried three header formats: RFC and ADR used one key per blockquote line;
   SPEC-1 used that shape while SPEC-2 through SPEC-4 used a pipe-delimited single line plus bold
   keys and a wrapped continuation. The third format had arrived in this repository's own corpus
   after header closure was specified — which is the failure mode RFC-10 describes, arriving in the
   corpus of the tool meant to prevent it. (Since converged onto `yaml-frontmatter` corpus-wide,
   ADR-33/ADR-42 — this historical drift no longer describes the corpus's current state, only what
   motivated choosing this as one of the two Phase A seed rules.)
2. **`Implements:` / `Derives-from:` resolves, and the target's status is reported.** Six pointers
   in `docs/specs/` currently name RFCs that are all `Draft`. That is RFC-12's
   decision-before-implementation gate with live subjects. Phase A only needs the pointer to
   resolve and the target status to be surfaced; whether a `Draft` target is an error, a warning, or
   the normal state of a young corpus is a decision for RFC-12, not for the checker.

Both are also the cheapest possible proof that the checker reads structure rather than greps text.

### Phase B — the rule set

The remaining rule families in §Rules, severity resolved from configuration (SPEC-3), and
`doctor`. Each rule lands with its planted-violation test and is exercised against `docs/` the day
it merges.

### Phase C — the second corpus

Part of the Phase 1 exit (`SPEC-1`), not Phase 0's. The falsifiable one. A second codebase runs green under the same binary with no code change, and its
hand-written linter is deleted. A failure here is a schema
finding to record rather than a code path to add.

### Phase D — the historical suite

SPEC-4's corpora and the documented bug classes. This is what turns "it works on two corpora" into
criterion 3, and it is last because it is the only phase whose blocker is not engineering.

## Discovery

**Default scope is what the VCS tracks**: `git ls-files` ∪ staged paths, intersected with the record
directories configured in SPEC-3. Never a raw directory walk.

This is not a preference. One source implementation globbed the filesystem directly, and a single
untracked scratch file under the records directory failed *every* run from that checkout, including
runs touching zero records. The tracked set is also what a reviewer sees, which makes the check's
input equal to the review's input.

A reference to a record that exists on disk but is not tracked is not a typo, and the report says so
(`BUG-13`). The CLI reads the files directly in each record type's directories that git does not track and
hands their ids to the rules, so `urzua-core` stays free of the filesystem. `pointer.resolution`,
`relation.supersession-reciprocity`, `relation.pair-reciprocity` and `sequence.unresolved-blocker` then
emit `pointer-untracked`, `supersession-untracked`, `pair-untracked` and `blocker-untracked` instead of
their `-unresolved` variant, with a message that names the file; `embodiment.locator-exists` emits
`locator-untracked` for a locator whose file is on disk and untracked, instead of `locator-missing`. A
missing target keeps `edit-field` `set`; an untracked one declares an exemption (see the action table),
because the repair is `git add` and `run-command` is closed to `urzua` commands. Tracking the file clears
the finding.

**Untracked records are the repository's choice** (`ADR-92`, `MILE-206`). The default above stands: a
repository that sets nothing governs tracked files only, and an untracked scratch file is not examined.
Two things follow from that being a choice and not a rule:

- **`discovery.include_untracked: true`** (`SPEC-3`) makes discovery also return the untracked,
  non-ignored files directly in a declared record directory that the type owns (the filename rules
  `load_records` applies). Git's own ignore rules decide "non-ignored" (`.gitignore`,
  `.git/info/exclude`, the global excludes), and a file outside every declared directory is never added.
  It is one function, `discover_tracked_files`, which gains the configuration as input, so `check`, the
  locator matcher, `init` and every other command read the same set. The scope source stays
  `tracked-sweep`.
- **Without it, nothing is silent.** A plain `check` adds one info notice, `discovery.untracked-skipped`,
  naming the untracked records under a declared directory that it did not examine: at most 20 paths and
  the count. It is emitted only when there is something to list, not for a record named on argv (examined,
  `BUG-24`), and not for a `--stdin-path` run; under a path argument only the records below it are listed.
  Info does not gate, so `fail_on: warning` is unaffected.

With the option on, a directory argument examines the untracked records below it, because the sweep it
scopes includes them; with it off the directory case is the one `BUG-24` pins. A named file keeps its
`BUG-24` behaviour whichever way the option is set (source `argv`).

Explicit paths on argv override discovery and are used as given -- for a directly-named file. A
directory argument still scopes the tracked-only sweep rather than walking the filesystem raw
(`BUG-24`): the "never a raw directory walk" guarantee above governs a directory argument the same as
the default sweep, and only a path naming one file precisely bypasses the tracked-set filter.

A file named in its type's `exclude_files` (`SPEC-3`, `MILE-150`) is dropped at this step: it is never
loaded as a record, so it is in no rule's population, `files_examined` and the type's file census for
`filename.identity-pattern`.

A type owns its `dir` and each of its `extra_dirs` (`SPEC-3`, `MILE-130`): discovery loads the files
directly in any of them as that type's records, so the type's ids form one namespace for pointer
resolution, `identity.collision` and `filename.identity-pattern`.

| `scope.source` | Meaning | `base` | Produced today |
|---|---|---|---|
| `tracked-sweep` | the tracked record set, plus the untracked record files when `discovery.include_untracked` is set | `null` | **yes** |
| `git-diff` | changed against a base ref | the ref | **yes** (`MILE-123`) |
| `argv` | a directly-named file not in the tracked set | `null` | **yes** (`BUG-24`) |
| `stdin` | one record's proposed content read from stdin | `null` | **yes** (`MILE-191`) |
| `none` | nothing selected | `null` | no |

The values are a serde-renamed `ScopeSource` enum (`BUG-25`), not a rendering of whichever internal
type produced them. Only variants the tool can actually emit exist on that enum; the rest of this
table is the declared vocabulary those modes will use when built, and is marked so a consumer is not
misled into branching on a value no run produces.

`base` is always present in `scope`: the ref as the caller gave it for a `git-diff` run, `null` for
every other source. So are `include_untracked` (a boolean: whether the configuration set
`discovery.include_untracked`) and `untracked_included` (how many untracked records that option added
to the run's files, `0` with it off), so a consumer can tell the two corpora apart.

**`stdin` scope** (`MILE-191`, `ADR-78`) is selected by `urzua check --stdin-path <path>`, the
prospective-content form of `check` that `fix --stdin-path` is for `fix` (`SPEC-8`). It reads the
proposed content of one record from stdin and judges it against every enabled rule that can judge one
record, writing nothing; it is the call the editor hook relays.

- **The proposal replaces the on-disk record** (or adds one: the path need not exist yet) before any
  rule runs, in the records and their text. `urzua-core` is handed that data like any other; the stdin
  read, the path and the git calls stay in the CLI. References still resolve against the rest of the
  corpus as it is on disk.
- **Only that record is reported on.** `files_examined` is `1`, and a finding is kept only when it is
  about the proposed record's file. A finding anchored to the configuration, a source file or another
  record is about something the edit does not touch and is left to `check`.
- **A rule that cannot report on a record is not run.** An enabled rule whose findings are anchored
  outside the records (`Config`, `Outside`, `Directory` or `ChangedPath` in the rule table) is listed
  in `rules_executed` as `not-applicable`, never `ran` with nothing found. This is also what keeps a
  single-record run to about half a second on this corpus.
- **A rule that compares with a committed copy is judged against HEAD's.** The proposed record is
  the one changed record, and its committed copy is the record at `HEAD`: `status.transition-allowed`
  judges the proposed `Status` against HEAD's, and `revision-log.version-bump-required` judges a row the
  proposal adds against HEAD's revision log and version. A record `HEAD` does not have is new: it has
  nothing to compare with, so the transition rule starts it at an initial status and the bump rule leaves
  it unexamined. `filename.identity-pattern` and `type.record-outside-declared-dir` are not judged here
  (they are about a directory or a path, not the content), and are left to `check`.
- **Drift is computed for that record only**, from the proposed text, as `fix --stdin-path` does.
- **A path no record type governs** is `not-run` (exit 2) with an `error` naming it, as is a run with
  no enabled rule that can look at the record. Combining `--stdin-path` with `PATHS` or `--base` is
  `not-run` (exit 2) too, not a flag ignored in silence.
- Severity, waivers, `fail_on` and the exit codes are those of a full run: an unwaived error exits 1.

**`git-diff` scope** (`MILE-123`) is selected by `urzua check --base <ref>`. It is a reporting scope,
not a reading scope, exactly as a path argument is (`BUG-60`):

- The changed set is every path that differs between the merge base of `<ref>` and `HEAD` and the
  working tree, so a branch is not charged for what its base gained since the fork, and staged and
  unstaged edits to tracked files count. Untracked files do not (the tracked-only guarantee
  above), except the untracked records a repository includes with `discovery.include_untracked`: they
  have no merge-base copy, so they are in the changed set and the added set exactly as a staged addition
  is, and the diff rules (`scope: changed`) judge a draft the way they judge a staged one (`ADR-92`). A
  rename is a deletion plus an addition, and a deleted path is simply not a record any
  more. A tracked record missing from the working tree that nothing has staged for deletion (an `rm`,
  or a `mv` before `git add`) is treated the same way, as removed, and a `missing-record` warning
  notice names it, so a record never vanishes in silence and the run is not `not-run`.
- The **examined set** is the changed records plus their dependents, and `files_examined`,
  `records_read_by_any_rule` and the findings are limited to it. A dependent is an unchanged record
  that (a) names a changed or deleted record's id in any header field (a pointer, narrative or
  relation field), (b) shares a changed record's id, or (c) cites changed or deleted evidence in
  its `Realized-by` (a glob locator such as `src/**/*.rs` counts as citing every changed file it
  matches, with the matcher locator resolution uses, not a path prefix), or (d) cites a normalised `Realized-by` locator that a changed or deleted
  record also cites, so a cross-record finding such as `embodiment.locator-promotion-candidate`
  lands on the same record a full check reports it on, or (e) is the `Scope` (or any record, for
  `"*"`) of a waiver that changed or was deleted, as it was at the merge base or as it is now, since
  that waiver may have stopped waiving a finding on it. A header value is split on commas first, so
  `ADR-1,ADR-3` names both. Deleted records are read from the merge base, since they are no longer in the
  tree; a rename is a deletion of the old id plus an addition. Without dependents a change
  elsewhere that breaks an unchanged record -- a deleted target, a renamed id, a changed `Status`,
  a reciprocal supersession, drifted or deleted evidence -- would be hidden, and a gate that hides
  what a full check reports is not a narrower check but a wrong one. One hop only: a dependent's own
  dependents are not pulled in.
- Reference resolution and every other corpus-wide computation still index the whole tracked
  corpus, so an unchanged target is never reported missing. Whether a finding survives a path argument or `--base` is decided by the
  anchor its rule declares in the rule table, never by rule id: a finding on the configuration, or on a
  file no record-type `dir` covers (a claim file, a source file, an index artifact, a changed path),
  survives every scope; one on a record follows the record; one on a tracked path or a type's directory
  follows the changed paths (`BUG-86`, `BUG-121`).
- A change to the configuration file the run loaded puts **every record** in scope (the rules
  themselves changed), and `scope.whole_corpus_reason` says so. A configuration that is a symlink
  counts as changed when either the link's own path or its target's path differs. A path argument
  still narrows this afterwards. A changed file below a declared dir that no type owns keeps its
  `type.record-outside-declared-dir` finding, though it is not a loaded record.
- `scope` carries the account of the examined set: `changed_paths` (every path git reports,
  deletions included), `dependents` (the unchanged records pulled in) and `whole_corpus_reason`
  (`null` unless the configuration changed). These fields appear only on `git-diff` runs.
- Path arguments still apply and narrow the examined set further.
- A diff with no changed record and no configuration change examines nothing and reports `ok`
  (exit 0, `files_examined: 0`), not `not-run`, provided some rule is enabled and looked at
  anything (a configuration with no rule enabled is `not-run` here exactly as in a full run): the question asked had the answer "nothing to
  judge". It is told apart from a whole-corpus pass by an info `notice` and by `scope`
  (`changed_paths` is empty, or holds no record), and a blocking finding that survives the scope
  (about the configuration) still makes it `findings-present`. A ref that does not resolve, or
  one with no merge base with `HEAD` (the error says "no merge base"), is a could-not-run (exit 2),
  never an empty selection. In a shallow clone the missing merge base is reported as a shallow
  clone, with the fix (fetch full history, e.g. `git fetch --unshallow` or `fetch-depth: 0`), not as
  unrelated histories.
- `--base` together with a directly-named file git does not track is refused (exit 2): the diff
  lists tracked paths only, so the file could not be examined, and dropping it silently is the
  failure `BUG-24` fixed. The refusal applies only to a file the run would not include: with
  `discovery.include_untracked` a named untracked record under a declared directory is in the diff as
  an addition and is examined (`ADR-92`).

A `null` base is not "no scope" — a full sweep legitimately has one. Any consumer keying "did this
run" on a missing base is reading the wrong field, and the sources exist so it does not have to.

**No git repository**: there is no fallback. A directory walk has the failure mode above, so discovery is
the tracked set or nothing. Outside a git repository, every command that reads one (`check`, `audit`,
`explain`, `graph`, `export`, `fix`, `index`, `refs`, `migrate`, `init`, `doctor`, `new`) exits 2 with
`{"status":"not-run","error":"urzua needs a git repository: ..."}` (`BUG-170`), whether or not a config and records
sit on disk. `rules` prints the static rule table and reads no repository, so it runs anywhere. The
`scope.source` table has no directory-walk value for the same reason.

## Rules

Rules are data where possible, not code (`urzua-core`). Each rule declares an id, the record types
it applies to, the **statuses it applies to**, and its default severity.

Status applicability is part of the declaration rather than an `if` inside each rule. A record can
legitimately hold a number without stating a decision — a tombstone (RFC-13) — and a check that
presupposes a decision is not wrong, it is *scoped*. A corpus that cannot express that forces its
records to invent content or its linters to be switched off.

### Shipped, by actual rule id

This is the complete, current set — generated from `urzua_core::rules::RULE_TABLE` by
`scripts/generate-rule-table.py` (`BUG-52`), never hand-edited, so it cannot drift from
the rule table the way its hand-maintained predecessor did. Regenerate with `make rule-table`; `make
ci` fails if the committed table is stale. Not a delta on top of the aspirational language below,
which stays only for what genuinely isn't built yet.

<!-- rule-table:start -->
| Rule id | What it checks |
|---|---|
| `header.required-fields` | Each type's `required_fields` (SPEC-3) are present; a missing header region or a duplicate key is also reported here. |
| `pointer.resolution` | A type's config-declared `pointer_fields` ∪ `narrative_fields` (MILE-90/ADR-44) resolves to a real record when it names one — a dangling reference is a finding, a resolved one is silent (`pointer.target-status` and `narrative-field.stale` separately judge what a resolved target's status means). No hardcoded field list — an undeclared type is skipped, not defaulted. A reference to a record whose file exists on disk but is not tracked by git is told apart from one that exists nowhere (`BUG-13`): a different finding variant, naming the file, because discovery reads only tracked and staged files; the same holds for `relation.supersession-reciprocity`, `relation.pair-reciprocity` and `sequence.unresolved-blocker`. |
| `pointer.target-status` | A reference resolves, but its target's `Status` is one the repository declared unacceptable (`not_in`). The optional `fields` option restricts it to the named declared pointer/narrative fields, so a dependency field (`Implements`) can be judged apart from a lineage field (`Derives-from`); an excluded field is not part of the rule's population, and a `fields` entry naming no declared field is a config error. Split from `pointer.resolution` in `MILE-80`. |
| `field.quality` | Field presence with real semantics: `blank`, `placeholder`, and `pending` are three distinct states, not one — the single most-repeated bug class in hand-written record linters. An absent field is never silently readable as any of the three. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `template_placeholders` option (`BUG-22`), off unless declared `true`, reads a field whose value is identical after trimming to its type's template text for that field (`.urzua/templates/<type>.md`) as a placeholder, and stops applying the compiled-in placeholder tokens; a type with no template is judged by its declared `pending` vocabulary alone, and declaring the option when no type has a template is disclosed as a warning notice. |
| `field.pending` | A required field is marked `Pending` — work declared unfinished, as distinct from forgotten. Split from `field.quality` in `BUG-38`. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. The optional `pending_until_required` option (`MILE-169`), off unless declared `true`, treats a pending value as legitimate until the record reaches a status the type's `statuses.require` ties the field to: a deferred slot stays eligible and is counted unexamined, and a field no `require` entry names is judged as before. |
| `claim.status-agreement` | A file outside the corpus claims to close a record whose own `Status` disagrees. Scans `claim_paths`; `closed_statuses` declared. A claim is a closing verb followed by a record reference in prose (`Fixes BUG-60`), which is a deprecated convenience: a claim written without the verb is not seen. The optional `closes` option (`ADR-85`) names a frontmatter key in which a claim file lists, on one line, the records it closes (`closes: [BUG-60, BUG-61]`, a single id, or `[]` for none), and that data is checked instead of a sentence: a listed id that resolves to no record is `claim.id-unresolved`; a listed record not at a terminal status its own type declares (the type's `statuses` success and failure, not `closed_statuses`) is an error naming the record, and offers an `edit-field` on its `Status`; a value that is a block list, a list continued over several lines or not a list of record ids is `claim.field-malformed`, never guessed at. Only the frontmatter block is read for it and every other key is ignored, so a release tool's own keys are untouched. The optional `strict` option (default `false`, needs `closes`) makes a claim file that has a frontmatter block and lacks the key a finding (`claim.field-missing`); `closes: []` declares that it closes nothing. A file with no frontmatter block is not a fragment: counted out of scope in the population and named in a notice, never given a finding; the exception is a file whose `---` fence is never closed or has a blank line before it, which the release tool refuses to read, and which under `strict` is `claim.field-malformed`. With `closes` declared the closing-verb reading still runs over the body, and `check` prints how many claims it found, unless the optional `verb_claims` option (`ADR-88`, default `true`, needs `closes`) is `false`: then no prose is read for a claim, only the field is, and `check` states that the reading is off (and, without `strict`, that a fragment lacking the key carries no claim). Without `closes` nothing changes. |
| `filename.title-consistency` | The filename's display number and the H1's display number agree (ADR-3). The H1's id is read the way a filename's is (`BUG-32`): with a declared `identity` pattern, the leading token must be the pattern's `prefix` and a number (`SPEC-3`); with none, a leading `PREFIX-N` or a bare `N`. A title that does not match, such as `# 2026-09-16 Notes` or `# v1.2 Release`, is reported as carrying no record id and never read as a number, and a number too large to compare is reported as such. |
| `relation.supersession-reciprocity` | If A supersedes B, B points back; status-aware (a claim only binds once the claiming record is itself terminal-accepted). Shared with `audit` (ADR-30) — `check` runs it too, not only `audit`. |
| `relation.pair-reciprocity` | For each declared `relation_pairs` entry with `reciprocity: required` (`RFC-47`, `MILE-122`): a claim `A.forward = B` is echoed by `B.inverse = A`, and a claim `A.inverse = B` by `B.forward = A`, across record types where the pair allows it. A pair that declares `types` is carried by those types only, and a claim to a type outside the list is excluded as out of scope (`MILE-155`). A pair that declares `provisional: {statuses, level}` reports an unechoed claim from a claimant at one of those statuses at the provisional level (default `warn`), and any other claimant at the rule's level (`MILE-154`). The optional `provisional.acknowledged_by` names a field on the peer that settles a provisional claim by naming the claimant (`MILE-163`); it is not read for any other claimant. Needs `relation_pairs`; enabling it with none declared is a load error. Beside, not replacing, `relation.supersession-reciprocity`, which keeps the combined `Supersedes / Superseded-by` key. |
| `relation.cross-type-disallowed` | A claim in a declared pair's field whose target is a different record type, for a pair declaring `cross_type: false` (`RFC-47`, `MILE-122`). Such a claim is excluded from `relation.pair-reciprocity` and reported here, never skipped silently. |
| `sequence.cycle` | A cycle in a declared `relation_pairs` entry that sets `acyclic: true` (`RFC-48`, `MILE-27`): following the written claims of either field, in the direction each declares, returns to the record it started at. Every record on a cycle is reported with the cycle's path, including a record that names itself and a cycle that crosses record types. The pairs `roadmap.pairs` names are read as one graph (`ADR-82`, `MILE-192`), so a loop that uses an edge of each of two declared pairs is reported on every member, and the message names the pairs it crosses; any other acyclic pair, and every pair when `roadmap.pairs` is absent, is read on its own. A claim whose target does not resolve is `sequence.unresolved-blocker`'s. Needs an acyclic pair; enabling it with none declared is a load error. |
| `sequence.unresolved-blocker` | A value in either field of a declared `relation_pairs` entry that sets `acyclic: true` and names no record (`RFC-48`, `MILE-27`): a reference that resolves to no discovered record, or an entry that is not, as a whole, one record reference (`MILE-4 MILE-5` is prose). A claim in a field that is one of the type's `pointer_fields` is left to `pointer.resolution` and `header.pointer-field-clean` when those are enabled. A fact that is not yet a record is a record of its own, not a prose value (`ADR-76`). Needs an acyclic pair; enabling it with none declared is a load error. |
| `sequence.status-agreement` | A record's written `Status` contradicts the derived `blocked` value (`MILE-144`, `RFC-48`, `ADR-83`). A record is blocked when any record it depends on, over the pairs `roadmap.pairs` declares and in either field, is not in a status its own type declares `success`; a dependency in a `failure` status, with no `Status`, or whose type declares no `success` blocks. Reports a blocked record that states a status its type lists in `statuses.progress` (`Done` included when listed; `Planned` is not progress), and a status the type lists in `statuses.blocked` while nothing blocks the record. Opt-in per type: only a type that declares `progress` or `blocked` is examined, and enabling the rule with no `roadmap.pairs`, or with no type declaring either, is a config load error. A value that cannot be computed is not guessed: a record in a `blocked` or a `progress` status that would be reported unblocked is counted unreadable when it names a dependency that resolves to nothing or to an unreadable record, or when a record that may write an inverse claim cannot be read, whether or not the type writes a `blocked` status at all (`ADR-83`). |
| `relation.target-status-undeclared` | A resolved pointer or narrative reference whose target type never declares `Status` at all — `ADR-60`'s gate would otherwise silently exclude it from every status-reading rule, with no disclosure of why (`RFC-45`/`ADR-63`). |
| `revision-log.change-class-required` | Every revision-log entry carries a `change_class` (ADR-14) from the type's declared vocabulary, by default `substantive` or `structural`; missing or unclassified is blocking, and the finding lists the accepted classes. A type may declare the vocabulary (`classes`) and the header names of its class and date columns (`class_column`, `date_column`) in `revision_log`; undeclared, the class is the last cell and the date the first (`ADR-94`). A log whose header lacks a declared column, or repeats it, is one finding on the record (`revision-log-column-unreadable`) and none of its rows is judged. A record of a type that declares `revision_log: {required: true}` with no revision-log section is a finding too (`BUG-50`); a type that does not declare it, or declares `false`, is not held to a log and its records without one stay unexamined. The log is the section a type's `revision_log` identity (default the line `**Revision log**`) opens, found as that line and never by a line that mentions the text (`MILE-167`). |
| `revision-log.version-bump-required` | A changed record that adds a revision-log row of a class its type declares must also change its version (`BUG-51`, `SPEC-18`: a substantive or structural change is a `Version` bump plus a revision-log entry). A type declares it as `revision_log: {version_bump: {field: Version, classes: [substantive, structural]}}`: a field name and the classes that require a bump (each one in the type's declared `classes` when it declares them), and no type has it by default. Judged against the merge base's copy of the record, so it reports `not-applicable` on a run with no `--base`, never clean; a record new in the diff, including one renamed in it, has nothing to bump from and stays unexamined. A row counts as added when its cells appear in the current log and not in the merge base's. A record whose log has a declared column missing or repeated, now or at the merge base, is reported unreadable in the rule's population, never clean and never read as an empty log (`ADR-94`). Enabling the rule when no type declares a bump is a config load error. |
| `embodiment.consistency` | A stated `Embodiment` agrees with the tier its `Realized-by` locators compute to (the tier order of the `embodiment` declaration, by default `test:` outranks `code:` outranks `spec:`); a locator that changed per git history since `Realized-by` was last touched overrides the expected value to the declared drift state unconditionally (RFC-5 tier 1, ADR-18/ADR-32). A record with no `Realized-by` field is judged on the back-pointers that cite it (`MILE-149`); one stating a state the declaration marks `claims_built` that has neither a `Realized-by` field nor a citing back-pointer is reported as a claim with no evidence, computed to the declared no-evidence state (`MILE-161`), and one stating any other value is not judged and stays unexamined. Requires full git history — a shallow checkout makes this rule silently unable to detect anything. Three opt-in refinements, all off by default (`MILE-129`): a locator written `path@sha256:<64 hex>` drifts when the file's content hash differs and not otherwise, in place of the git-history heuristic; the `drift_grace_days` option holds back reporting drift whose locator last changed fewer than that many days ago, and a record in that window agrees with either its tier or a stated drift state (the window never contradicts a record that already says so, and `urzua fix` leaves it alone); the optional `drift_audit_field` option names a declared date field (needs `drift_grace_days`): a record carrying a `YYYY-MM-DD` date there has its window counted from that last-audit date instead of from the cited file's last change, and a malformed date is reported and falls back to the file (`MILE-166`); the `claim_max_age_days` option warns when the `Realized-by` line was last touched more than that many days ago. The optional `sticky_states` option lists `Embodiment` values a human sets that no evidence can compute, such as `Deprecated` and `Inactive`: a record stating one is examined and never reported as drift or as an old claim, `urzua fix` leaves it alone, and a listed value that evidence does compute (a state of the `embodiment` declaration) is a load error because it would hide real drift (`MILE-148`). Source back-pointers (see `embodiment.back-pointer-resolution`) are evidence too: an `Implements:` citation is code tier and a `Verifies:` citation in a declared test file is test tier, equal to a `test:` locator (`MILE-146`). A record with `Embodiment` but no `Realized-by` field is judged on back-pointer evidence alone when one cites it, and a record neither cited nor carrying `Realized-by` is not judged (`MILE-149`); a finding names the evidence it computed from. |
| `embodiment.claim-expired` | A claim expires after a period declared per record type, status and evidence tier (`ADR-97`, `MILE-129`). Opt-in: a configuration that does not declare the rule runs nothing new and loads and reports as before. `periods` is a list of `{type, status?, tier?, days, spread_days?, level?}`; the most specific entry wins (type, status and tier outrank type and status, which outrank type alone), two entries of equal specificity that can match one record are a load error, and a record of a listed type that no entry matches is eligible and not examined, and an entry whose type holds claims (a record with a locator) and covers none of them is reported by `config.scope-matches-nothing`, while an entry for a type with no claim yet is not. The `tier` is the one the record's locators compute to under the `embodiment` declaration, so a milestone or bug that states no `Embodiment` is judged too; a record whose status is in the type's `claim_retained`, or that states a `sticky_states` value, is not. A claim's age is whole UTC days from its anchor: the date in the declared `anchor_field` (`YYYY-MM-DD`, written only by `urzua reverify`; a malformed or future date is reported and ignored), otherwise the date of the commit that last touched the first evidence line: the `Realized-by` line, or for a record with no value there the first Done-means item evidence clause (`ADR-97`, amendment of 2026-11-08). A record is judged when its `Realized-by` field or its Done-means item evidence clauses give any evidence; one with neither is not examined ("no locator field and no Done-means item evidence clause"). A record with no valid anchor date has its first period lengthened by a deterministic offset of 0 to `spread_days` days, the first 8 bytes of the SHA-256 of its `Stable-Id` as a big-endian number modulo `spread_days + 1`, so a corpus rewritten on one day does not expire on one day; the offset ends with the first valid anchor date and a record with no `Stable-Id` gets 0. A finding is reported at the entry's `level`, held to the rule's own, and offers `urzua reverify --ids <record>` (a review report, not a write). `require_pin` (evidence categories that must be pinned before a renewal) and `max_batch` (most records one `reverify --apply` names) are read by that command. Drift and expiry are independent: both report on one record, `drift_grace_days` does not delay expiry, and a waiver suppresses the finding without renewing the claim (a waiver with no `Expires` is governed by the `permanent` declaration, `ADR-86`). Declaring this rule together with `claim_max_age_days` on `embodiment.consistency` is a load error. |
| `embodiment.claim-renewal-unattested` | On a `--base` run, a changed record whose `anchor_field` (declared by `embodiment.claim-expired`) is newer than in its merge-base copy and which gained no `Re-verified` revision row dated that day, or whose `anchor_field` the merge-base copy carried and the branch removed (`ADR-97` section 6). Prevention is not claimed, detection is: a date in a text file can be typed, and only `urzua reverify` writes the row that says who renewed the claim and what the tool checked. It cannot see a record that never carried the field, nor one new in the diff (no merge-base copy), and not-applicable on a run with no `--base`. Needs `embodiment.claim-expired` with an `anchor_field`; opt-in; its level is the repository's. |
| `embodiment.requires-accepted-status` | A record whose `Status` is not an accepted status states an `Embodiment` the `embodiment` declaration marks `claims_built` (`MILE-138`): a decision not yet made cannot have been built. The accepted statuses are the type's declared `statuses.success`; a type declaring none uses the rule's `accepted_statuses` option, and with neither the type is not examined. Any other state, including the drift state and a sticky one, is not flagged, and a record missing either field is left to the header rules. Opt-in; the population counts the records the rule could judge. |
| `embodiment.back-pointer-resolution` | A source comment line `Implements: ADR-12` (a comment leader such as `//`, `#`, `--` or `*`, then the marker, then comma- or space-separated record ids, each optionally followed by a note, as in `Implements: ADR-1 (why), ADR-2`) in a file under a `back_pointer_paths` entry counts as code-tier realization evidence for the record it cites, in `embodiment.consistency` and `urzua fix` (`MILE-128`, narrowing `RFC-5` section 2), and a back-pointer citing a record that does not exist is reported here. The marker is read at the start of a comment line or in a trailing comment after code (`//`, `#`, `--`, `/*`, `<!--` after whitespace); the marker is case-sensitive; reading continues past a note to the next comma-separated id and stops at a segment that does not open with one; ids are compared numerically. Opt-in and needs `back_pointer_paths` (path prefixes, normalised at config load, no globs); evidence is collected only while this rule is enabled. A declared path covering no tracked file is itself a finding, and a non-text file is counted as unreadable, never as clean. The optional `test_paths` option (path prefixes, each inside a `back_pointer_paths` entry, else a load error, or globs such as `**/*.test.*` matched among the scanned files; a glob that cannot compile is a load error and one matching no scanned file is a finding) declares the test class: a `Verifies: ADR-12` comment in a file under it is test-tier evidence for the record it cites, and is checked for resolution like an `Implements:` one; a `Verifies:` comment elsewhere is not read (`MILE-146`). |
| `embodiment.back-pointer-decision-accepted` | A source back-pointer (`Implements: ADR-12`, read exactly as `embodiment.back-pointer-resolution` reads it) citing a record whose `Status` is not one at which implementation is permitted is an error: code that says it implements a decision is landing ahead of that decision (`MILE-133`). Permitted means the cited type's declared `statuses.implementable` (a terminal `success` status is not assumed to permit it); a type that declares none uses the rule's `accepted_statuses`, and with neither the status cannot be judged, which is reported rather than passed. Needs `back_pointer_paths`; `accepted_statuses` is optional. On a `--base` run only back-pointers in files the diff changed are judged (file granularity). A citation of no record is `embodiment.back-pointer-resolution`'s, not this rule's. A declared path covering no tracked file is itself a finding. Mechanical fixes carry no back-pointer and are outside it. It checks the cited record's status, never whether the code matches the decision. |
| `embodiment.locator-exists` | A `Realized-by` locator names a path that is not both git-tracked **and** present on disk, is empty, does not parse, or carries a scope (`#symbol` or `#L<a>-L<b>`) that does not resolve in the file, a scope without a pin, or a commit anchor (not yet supported). Tracked alone passes a staged deletion; on disk alone passes a gitignored file. A locator that resolves outside the repository (absolute, `..` or a link out) is never probed on disk, so it reads as not in the working tree, and a Done-means item that carries a valid waiver is not examined (the waiver holds while its evidence does not resolve). A locator naming a file that exists on disk but is not tracked by git is a separate variant from one naming nothing (`BUG-13`), because the repair is to track the file. A comma separates `Realized-by` entries only when a category prefix (`spec:`, `code:`, `test:`) follows it; any other comma is part of the path (`BUG-129`), and a path that itself contains `,<category>:` is read as two entries. The `locator_kinds` option (`file`, `directory`, `glob`; default `[file]`) declares which kinds a locator may be: a directory or glob is legal when it matches at least one tracked file present on disk, and one of a kind not declared is reported naming the option. The `pinnable` option (default `[file]`, each kind also in `locator_kinds`) declares which kinds may carry a `@sha256:` pin (`BUG-176`). |
| `embodiment.locator-promotion-candidate` | The same locator cited by more than one record's `Realized-by` is a promotion candidate, surfaced as a finding, never auto-promoted (ADR-18). The optional `scope: changed` option (`ADR-90`) makes it a nudge for the moment someone touches a record that shares a locator: a locator is reported only when a record a `--base` diff adds or changes cites it, with the finding on that changed record and every citer (changed or not) named in the message; none on a plain `check`, and the slots it did not judge are counted in the population's `deferred_by_scope`, with one `info` notice saying why. |
| `header.field-set-consistency` | Every header field belongs to its type's `required_fields` ∪ `known_fields`, when `known_fields` is declared (ADR-39). The optional `allow_extension_prefix` option (`MILE-141`), off unless declared `true`, exempts header keys beginning `x-` in any case; an unprefixed undeclared key is still a finding. The optional `extension_prefixes` option (`MILE-141`) maps a record type to its own prefix, used for that type in place of `x-`; a type it does not name keeps the rule-level behaviour. A prefix is non-empty, holds no whitespace and does not start a field the type declares. |
| `field.untrimmed-value` | A field value carrying leading/trailing whitespace, reachable only through `yaml-frontmatter` — disclosed rather than silently trimmed. |
| `field.leading-reserved-indicator` | A declared field's value starts with a reserved YAML indicator character (`@`, `*`, `&`, `!`, `%`, `|`, `>`), which needs quoting to parse at all — the same avoidable, decorative-prefix hazard `BUG-18` found for `Author`/`Deciders`, generalized so the next instance is caught rather than found by hand. |
| `header.field-case-mismatch` | A declared field written under a different case than its declaration (`RFC-40`/`ADR-58`) — the one place a case-only miss is distinguished from genuine non-adoption. |
| `header.case-variant-key` | One record's header writes two keys differing only by case (`Status`/`status`), reported as the fact of the difference, never as which spelling is right (`MILE-113`). Opt-in and declaration-free: it reads the keys a record wrote, not a declared vocabulary, and composes beside the exact-match rules without changing them (`ADR-57`). An exact repeat of one spelling stays `header.required-fields`' finding. |
| `narrative-field.stale` | A type's config-declared `narrative_fields` pointer (e.g. a milestone's `Blocked-on`) resolves to a target whose `Status` has reached a terminal state (e.g. a cited bug is now `Fixed`) — a signal to re-examine (ADR-42, generalized beyond `Blocked-on` by MILE-90/ADR-44). `terminal_statuses` is declared per repository. |
| `type.dir-matches-nothing` | A type's declared `dir` does not exist at all (`BUG-56`'s hazard for `dir`, no load-time guard). An existing, empty directory is a legitimate declared-but-unused state and is not flagged. Also reports a type’s `exclude_files` entry that names no tracked file directly in its `dir` (`MILE-150`), so a stale exclusion is visible. Reports each `extra_dirs` entry that does not exist (`MILE-130`). Reports each status a type's `sections.exempt_statuses` lists that none of the type's records holds (`MILE-7`), so an exemption that sets nothing aside is visible rather than silent. |
| `identity.collision` | Two records resolving to the same identifier — a corpus error the index cannot represent, reported rather than papered over (`BUG-79`). |
| `identity.open-pr-collision` | A record a `--base` diff adds shares its type and number with a file on a fetched pull-request head ref under a different `Stable-Id` (`ADR-93`, `MILE-89`): two branches filed the same number from one `main`, and each looks valid alone. The refs are the ones `urzua new` fetches into `refs/urzua/pr/<segment>/*` with `new.check_open_prs`; `check` never fetches, so with none present the rule notices how to fetch them and its population counts zero refs, never a clean run over nothing. A head whose file for that id carries the record's own `Stable-Id` is the same record and is not a collision, so a pull request is not flagged against itself nor against its own merged copy. Plain git cannot tell an open pull request from a closed one, so a closed, unmerged head can still hold a number a new record legitimately took: the finding is a warning, which still gates under `fail_on: warning`, and the path out is a waiver whose `Reason` names that pull request, or renaming the file and its H1 to the next free number (the one `urzua new` would take; `urzua migrate ids` cannot renumber). Judges the changed set: `not-applicable` on a run with no `--base`. Opt-in. |
| `type.record-outside-declared-dir` | A record-shaped tracked path that no type's `dir` claimed (`BUG-62`) — a config written before `RFC-35` silently loses coverage on upgrade otherwise. |
| `type.no-declared-spec` | Which configured types currently have no `spec` pointer declared at all (ADR-41/ADR-43) — a signal to review, never a mandate; a type can permanently have none declared. |
| `header.pointer-field-clean` | A type's `pointer_fields` entries are clean, comma-separated references only, format-enforced (MILE-90/ADR-44) — an empty entry between commas or freeform trailing prose is flagged. Never applies to `narrative_fields`, which tolerate prose. |
| `config.companion-declaration-missing` | A type's `companion_fields` lists pairs of relation roles, for example `[embodiment_state, embodiment_locator]`, whose fields it must declare together (`BUG-41`). A type declaring one field of a pair in `required_fields`/`known_fields` without the other is reported, as is a pair whose fields it declares neither of: a rule reading both, such as `embodiment.consistency`, cannot judge a record that lacks one. The fields are the ones the type's `relation_fields` names for the roles; no pair is known to the engine. |
| `config.pointer-declaration-missing` | A type declaring either `pointer_fields` or `narrative_fields` must declare both explicitly, even as `[]` (MILE-90/ADR-44) — omitting one is not the same as declaring zero fields of that kind. |
| `config.pointer-field-not-known` | Every field named in a type's `pointer_fields`/`narrative_fields` must also appear in that type's own `required_fields`/`known_fields` (MILE-90/ADR-44) — otherwise `header.field-set-consistency` would never have heard of it. |
| `config.pointer-narrative-overlap` | A field must be exactly one kind: named in both `pointer_fields` and `narrative_fields` for the same type is a contradiction (MILE-90/ADR-44). |
| `config.header-none-has-no-required-fields` | A type declaring `header_shape: none` has nowhere for a field to be, so a non-empty `required_fields`, `known_fields`, `pointer_fields`, `narrative_fields`, or `relation_fields` is a self-contradiction (`ADR-50`). |
| `config.relation-field-not-known` | Every field named in a type's `relation_fields` (`RFC-42`/`ADR-61`), including the `date` and `author` fields `urzua new` fills (`MILE-176`), must also appear in that type's own `required_fields`/`known_fields`, the same shape as `config.pointer-field-not-known`. |
| `config.known-fields-declaration-missing` | A type declaring no `known_fields` at all gets no field-set governance from `header.field-set-consistency` (`ADR-53`'s "declared, not voted" default) — opt-in, so a repository can require the choice be made explicit rather than left ambiguous by omission (`RFC-43`/`ADR-62`). |
| `config.schema-version-outdated` | The configuration file is at the previous `schema_version` and was read by upgrading it in memory, so no rule saw the old shape (`ADR-77`, `SPEC-3`'s "Supported schema versions"). One finding on the config file naming the version found, the current version and the command that rewrites the file, `urzua migrate config`, plus the keys the upgrade dropped. A config at the current version is examined and clean. Opt-in like every rule, and each upgrade step enables it, so a config upgraded once is reported by the next release that moves the version again; under `fail_on: warning` it fails a strict repository until the file is migrated. |
| `filename.identity-pattern` | A file directly in a type's `dir` whose name does not match the type's declared `identity.pattern` (or, for a type declaring none, the `TYPE-NNNN-slug.md` convention) is reported rather than left out of every rule's population (`MILE-120`). When every file in the directory is off-pattern, such as a bare-number corpus (`0001-slug.md`), it is one finding per type naming the expected file name for each file, since renaming is the supported route (`RFC-46` was rejected). The census counts each file examined, and omits a file the type declares in `exclude_files` (`MILE-150`). |
| `status.undeclared-value` | A record's `Status` is not one of the values its type declares in `statuses.values` (`MILE-132`). A type declaring no `statuses` is not examined, and a record with no `Status` is left to `header.required-fields`. |
| `status.required-field-missing` | A field the type's `statuses.require` ties to the record's current `Status` is blank, unedited template text, or (when the type declares `statuses.pending`) one of that declared placeholder vocabulary's whole values such as `Unknown` or `none recorded`, or a value matching one of its `{pattern: ...}` entries for a placeholder family (`MILE-132`, `MILE-153`, `MILE-162`), for example `Deciders` on an `Accepted` record. A type declaring no `pending` keeps the old reading, so a value that only contains the word is still a real value. A type declaring no `require` entries is not examined. The optional `field_levels` option (`MILE-140`) sets `warn` or `error` per field; a field it does not name takes the rule's `level`. |
| `status.forbidden-field-present` | A field the type's `statuses.forbid` bars at the record's current `Status` carries a value (`MILE-132`), for example an acceptance date on a record that is not accepted. A type declaring no `forbid` entries is not examined. |
| `status.transition-allowed` | A changed record's `Status` moved along an edge its type's `statuses.transitions` does not list, comparing the status the merge base committed with the one in the change under check, or a record new in the change starts at a status outside `statuses.initial` (`MILE-144`, `ADR-72`). Needs `--base`: a whole-repo run reports it not-applicable, not a pass. A type declaring no `transitions` is not examined, a record the diff did not change is eligible and not examined, and a record whose committed status cannot be read or is no longer declared is counted unreadable. A path the merge base lacks reads as new; a base copy that exists but cannot be read (not UTF-8, a missing object) is not new: it is counted unreadable and a warning notice names the error, so a read failure cannot skip the transition check. A record added at a path the merge base lacks is matched by `Stable-Id` to a record the diff deleted, so a renamed record is judged against the status it had before and not called new (`BUG-191`); one with no `Stable-Id` or no match reads as new, and one matching several deleted records is counted unreadable. A repository whose ADRs and RFCs enter at `Proposed` or `Draft` and are accepted in a later change declares that as `initial`, so a record filed already accepted is reported; this repository does so. |
| `status.unreachable-state` | A status a type declares in `statuses.values` that no path of its `statuses.transitions` reaches from `statuses.initial` (`MILE-144`, `ADR-72`): no record can legitimately hold it. Judges the declaration, not a record, so it needs no `--base`. A type declaring no `transitions` is not examined. Enable it at `warn` for the informational reading. |
| `status.delivery-agreement` | A record's `Status` agrees with its delivery evidence, in both directions (`MILE-183`): the tier computed from its `Realized-by` locators and source back-pointers (drift ignored) is one of the type's `statuses.delivery.delivered_tiers` while the `Status` is one of its `not_done` statuses, so a built record still reads as unbuilt; and a `success` status whose evidence computes to one of its `unevidenced_tiers`, so a record claims to be done with nothing behind it. The mapping is the type's own declaration (`delivered_tiers`, `unevidenced_tiers`, `not_done`, `set_status`); the rule knows no status or type by name. `urzua fix --apply` writes `set_status` for the first direction; the second has no single correct value and is repaired by adding evidence. `unevidenced_tiers` may be empty, and the type then never reports the second direction. `back_pointer_evidence: false` (default true) makes a source or test back-pointer that cites the record not count as evidence, so only the type's evidence field is read (`ADR-84`). The rule applies to any type that declares `delivery`, whatever the type is called. A type declaring no `delivery` is not examined. The population counts the records of types that declare it. |
| `status.done-items-shape` | A Done-means section that does not follow the item grammar of `ADR-96` section 1, on a type that declares `statuses.delivery.items`: a prose-only section (no list item), a list marker indented less than the item indent, a clause on a line that belongs to no item, clauses on the first line and on final lines at once, a waiver clause before the evidence clause, an evidence clause that is not entirely locators (backticks or quotes before the first `category:`, whitespace inside a locator, no locator at all), a leading `[x]` or `[ ]` checkbox, and a waiver clause with an unknown category, an empty or missing reason, a missing, malformed or unneeded cite or date. A clause that does not parse is a finding, never skipped, and its item reads as unresolved. A marker word with no clause separator before it is plain text. A `Kind` value outside the type's declared `delivery.items.kind` vocabulary is a finding naming the allowed kinds, and the record is then judged as one with no kind. A type declaring no `delivery.items` is not examined. The population counts the records of types that declare it. |
| `status.waived-item-stale` | A waived Done-means item whose waiver has outlived its reason (`ADR-96` section 3), on a type that declares `statuses.delivery.items`: the item also carries an evidence clause that now resolves (remove the waiver), the record its waiver cites does not exist, a `deferred-to` (a category declared `expires: required`) whose cited record is finished, or an expiry date that has come (from that date on). Every waived item is also listed by category in the report's notices, never omitted. A dated `WAIVER` record can waive this rule like any warning. A type declaring no `delivery.items` is not examined. The population counts the records of types that declare it. |
| `date.field-valid` | A value in a declared date field (`date_fields`) that is not a well-formed `YYYY-MM-DD` date, or is well-formed but not a real calendar date such as `2026-02-30`, is an error (`MILE-137`). A blank, placeholder or pending value is not a date value and is left to `field.quality` and `status.required-field-missing`. Needs `date_fields`. The population counts the date values examined. |
| `date.field-order` | Within one record, a date field that is later than the field `date_order` says must not be later is an error: `Date <= Accepted-on` reports a record accepted before it was written (`MILE-157`). Equal dates pass. A value that is blank, a placeholder, pending or not a real `YYYY-MM-DD` date is not compared and is left to `date.field-valid`, `field.quality` and `status.required-field-missing`, which own those states. Applies to every record type that declares both fields of an entry. Needs `date_order`. The population counts one slot per record and entry; a record missing either date is eligible and unexamined. |
| `citation.resolution` | A decision ID cited anywhere in a record's prose (`ADR-12`, with a configured type prefix, standing alone) resolves to an existing record, compared numerically so zero-padding does not matter (`MILE-136`). The header, which `pointer.resolution` governs, and fenced code blocks are not read; an inline code span is. Opt-in. The optional `external_ids` option (`MILE-156`) lists ids that share a record prefix but are not records, each an exact id (`RFC-4122`) or an inclusive numeric range written `PREFIX-LOW..HIGH` under a declared prefix; a matching citation is counted as eligible and unexamined, never resolved or reported, and not as out of scope, so `config.scope-matches-nothing` does not read a declared id as a misconfiguration. An entry under no declared prefix is a config error. Its population counts the citations, so a run over none is distinguishable from a clean one, and external citations are visible as eligible and unexamined. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that cites an id as it was is not reported; it is `false` by default (`MILE-156`). The optional `non_reference_field` option (`BUG-190`, `ADR-75`) names a header field in which a record lists, as comma-separated exact ids (`ADR-72`; no ranges, compared by value), the ids it names without citing as a reference: a citation of a listed id in that record only is counted as eligible and unexamined, neither resolved nor reported, with `external_ids` taking precedence. A listed id the record never cites in the prose the rule reads, and an entry that is not an exact id of a declared prefix, are findings on the record naming the field and its line. With the option set, those declared entries are judged in a population of their own (`MILE-186`, `ADR-75` clause 3): a second `rules_executed` entry for this rule whose unit is `field-entry`, with one candidate per entry a record declares (valid, uncited and malformed alike, all examined, none out of scope), and zero eligible when no record declares any; the uncited and malformed findings are counted there and not in the citation population. The field must be listed in a record type's `known_fields` and must not be a pointer, narrative or relation-role field, or the config does not load. `check` prints one aggregated Info notice naming declared ids that now match a real record; a notice never changes the exit code. |
| `citation.path-exists` | A repository path cited in backticks inside a `reference_sections` section (a `##` heading, matched exactly) exists in the working tree, tracked and on disk (`MILE-143`). A cited path is a code span with no whitespace, containing a `/` and ending in a file name with an extension or in `/`; a URL, a host-led path (`example.com/x`), an absolute or `~` path, a glob or placeholder (`*?<>{}$|`) and anything else is not a path and is not read. A trailing `:line` or `#anchor` is dropped, and `./` or `../` resolves from the record's directory. Fenced code blocks are not read. Needs `reference_sections`; a record without such a section is out of scope, and the population counts the paths cited. The optional `skip_revision_log` option, when `true`, leaves each record's revision log (the table under its type's declared `revision_log` identity) unread, so append-only history that names a path as it was is not reported. It is `false` by default. Offline and deterministic. Cited issue and pull request numbers are `citation.issue-exists`'s. |
| `citation.issue-exists` | An issue or pull request cited inside a `reference_sections` section (a `##` heading, matched exactly) is in the committed snapshot of the repository's numbers (`MILE-143`). `issue_refs` declares the `snapshot` file (relative to the repository root), the `repository` (`owner/name`) it describes and the citation `forms` read: `number` is `#123` (outside inline code, not `owner/repo#123`, not an anchor link `](#123)`), `url` is a `https://github.com/owner/name/issues/123` or `/pull/123` link to that repository; a form not declared is not read. A number is judged against the snapshot's issues and pull requests together, since GitHub shares one number space and redirects either URL. Offline and deterministic: `check` never reaches the network, and only `urzua refs refresh` writes the snapshot. A snapshot that is absent, unreadable, of another schema or of another repository is one finding on the snapshot path, and every citation is counted as eligible and unexamined: the rule never passes for want of its input. Each finding names the date the snapshot was generated, and `check` prints a notice with that date and the snapshot's size, so staleness is visible. The optional `skip_revision_log` option behaves as for `citation.path-exists`. The optional `issue_refs.fields` names header fields (as a type declares them, such as `Found-in`) whose values are read for the same forms, each finding naming the field and its line; a name no record type declares is refused at load, and a record without the field is not examined. Body prose outside `reference_sections` is deliberately not readable. The population's unit is the citation. |
| `boundary.published-path-reference` | Declared boundaries (`boundaries`: path globs with a `kind` of `published`, `vendored` or `generated`, a required `reason` and a `severity`; `published_paths` is read as one published boundary with no reason required, and is not deprecated) and what follows from each kind (`RFC-11`/`ADR-9`/`ADR-80`, `MILE-125`). A record identifier (any identifier carrying a configured type prefix, resolvable or not, in prose as well as in a marker) inside a `published` boundary is a violation, because it publishes a pointer no external reader can resolve; a back-pointer (`Implements:` or `Verifies:`) inside a `generated` boundary is reported, and a plain record identifier there is not; a `vendored` boundary is declared and prohibits nothing. A boundary's `severity` is the level of its findings and the rule's `level` only enables the rule. A boundary's `exclude` globs remove paths from its violation check only, and the declared `ignore_marker` excuses the one line it is on. An `exclude` entry, a boundary path or an `ignore_marker` use that matches or suppresses nothing is itself a finding, never a clean pass. The violation matcher is wide and the evidence collector's stays narrow: a back-pointer inside a `published` boundary is not realization evidence. An identifier split across lines is not detected. |
| `section.required` | Each `##` section a type's declared `sections.required` names is present in every record of that type (`MILE-4`, `RFC-17`), under its `name` or one of its declared `aliases` (`MILE-165`; `check` discloses each alias in use as an info notice), or, for a `y-statement` declaring `preamble: true` and a `marker`, in the text before the first `##`. A type that declares none is out of scope; a record whose status is in the declaration's `exempt_statuses` (a record that made no decision, `MILE-7`) is counted eligible and unexamined, not asked for the sections, and not out of scope, so `config.scope-matches-nothing` does not read a declared exemption as a misconfiguration (`BUG-187`). A required section may declare `level` and `under` to be a `###` (or deeper) heading nested under a named `##`, which needs the type's `sections.depth` to reach it (`MILE-98`). Presence only: it never judges what a section says. |
| `section.shape` | A required section that declares a `shape` has that shape (`MILE-5`, `MILE-121`): `y-statement` needs its TL;DR block to carry the five phrases `In the context of`, `facing`, `we decided`, `to achieve`, `accepting`, word-boundary matched, and reports a marker with no block as its own error; `table-min-rows` needs a table with at least `min_rows` data rows. A section found under an alias is checked too, and a `preamble: true` Y-statement is judged from the text before the first `##`. A missing section is `section.required`'s to report, not this rule's. Shape only, never meaning. |
| `index.freshness` | A committed index of one record type (a file, or a marked list inside a README, in a declared `format` with `{id}`, `{file}`, `{title}` and declared-field placeholders such as `{Status}`) lists exactly the records: a record with no entry, an entry naming no record, a duplicated entry, an entry whose file or title differs from the record's, and a line the format does not match are each an error (`MILE-139`). A `format` may span several lines (written with `\n`), reading one entry over that many non-blank lines; it needs `{id}` or `{file}` (a link list that names no id is identified by its file); `{title}` is the H1 without its leading number or the record's own id (`MILE-152`). A missing artifact, or one whose declared `begin`/`end` markers are absent, is an error too. Needs `indexes` (`path`, `type`, `format`, optional `begin`/`end`); `check` only compares, and `urzua index` regenerates the artifacts from the records. The population counts the artifacts compared, so a run over none is distinguishable from a clean one. |
| `diff.source-requires-record` | A `--base` diff changes a path under a declared `source_paths` entry (a prefix or a glob) but changes no governed record (a `.md` file directly in a record type's `dir`) (`MILE-124`, `MILE-158`). A changed path matching `exclude_paths` is not source and stays in the population unexamined, so a diff touching only such files (tests beside the code) is not a trigger. One finding per changed source path, so a waiver can scope to one. It asks only that a record changed, never that it is the right one. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Deleted paths count as changed. Declaring `decision_types` (each record type that holds decisions, with the statuses at which one counts as decided) turns it into the decision gate (`MILE-133`): the diff must change a decision record at a decided status, a record whose `link_fields` entry (a pointer or narrative field such as `Implements`) names one, or a record of a `fix_types` type (a mechanical fix needs a bug record and no decision); a changed decision still at an undecided status does not release it and is named in the finding. A deleted record satisfies nothing. `fix_types` and `link_fields` need `decision_types`, and every type, status and field a declaration names must exist, or the config does not load. Off unless declared: without `decision_types`, any governed record releases the rule. |
| `diff.source-and-records-mixed` | A `--base` diff changes both a governed record and a path under a declared `application_paths` entry, a prefix or a glob (less any `exclude_paths`), a prompt to split the change (`MILE-142`). One finding naming the first path of each kind, in path order. A governed record is never application source, whatever the prefixes say. Reports `not-applicable` on a run with no `--base`, never clean. |
| `diff.decider-required` | A `--base` diff moves a record to a status a gate of the top-level `authority` table names (`gates: [{type, to, by}]`, for example an ADR to `Accepted`), or changes a path a gate names (`gates: [{paths, by}]`), and the pull request's author, or a login whose approval is current for the head commit, is not in the gate's decider group (`ADR-89` section 4, `MILE-200`). The identities arrive as input and are never fetched (`check` does not use the network): `urzua check --actors FILE` reads a JSON file the gate action writes from the host API, `{"schema": 1, "pr_author": LOGIN, "approvers": [LOGIN, ...]}`, where an approval given before a later push is not listed. Which of the two counts is the table's `evidence` (`pr-author`, `pr-approver`, any of; `pr-author` unless declared, because a host bars an author from approving their own pull request and a single maintainer could then never satisfy `pr-approver`). A move is read against the merge base's copy of the record, the status-transition mechanism: a record new in the diff, and one whose base copy cannot be read, count as a move to the status they hold, and a record edited in place at a status it already held does not. One finding per gated record or path, naming the gate, the group and what the file says. **This rule proves authorship, not who decided.** With a single maintainer every pull request an agent opens under the maintainer's token is authored by a decider, so the rule passes and cannot tell an agent from the maintainer; it gives a recorded trail and, in a repository with a second human or a distinct agent identity, a check that means something. A move or change with no `--actors` file is reported as `info` on a local run (a pre-commit hook cannot know) and as an error at the rule's level under the option `require_evidence: true` (default `false`), so a missing input is never read as a pass. Judges the changed set, so it reports `not-applicable` on a run with no `--base`, never clean. Opt-in, and needs the top-level `authority` declaration. The population counts changed paths. |
| `override.unreviewed` | An editor-hook override that no one has reviewed is reported, on the committed override log the repository declares as `override_log` (`ADR-78`, `MILE-188`). The log is a Markdown table of `Date`, `Name`, `Record`, `Field`, `Rule`, `Reason` and `Reviewed-by`; the hook appends a row, with `Reviewed-by` blank, whenever a named override lets a blocked edit through, and the entry is reviewed once a person names themselves in `Reviewed-by` on its row in a later change; a blank cell, a placeholder (`TBD`, `-`) and a declared pending value are not a review. The finding names the override, the record, the rule and the reason, so it surfaces in the pull request that carries the log change. A row that is not an entry is a finding; a declared log that does not exist, or is not text, is reported and counted unreadable, never read as an empty log. Opt-in; needs the top-level `override_log` declaration. The population counts log entries. |
| `embodiment.locator-on-back-pointer-path` | A `Realized-by` locator names a file under `back_pointer_paths`, where an `Implements:` or `Verifies:` comment would say the same thing and is read automatically (`ADR-78` item 4, `MILE-189`, `RFC-7` section 5: prefer the lighter mechanism). A warning nudging the evidence into that comment, never an error (a locator whose file already carries the comment for the record is told to drop the locator, not to add the comment, and a record of a type declaring `statuses.delivery.back_pointer_evidence: false`, which counts no comment as evidence, is not nudged), because the two mechanisms are not interchangeable: only a `code:` locator on a file under `back_pointer_paths`, or a `test:` locator on a file under the optional `test_paths` (the only files whose `Verifies:` comment counts), has a comment of the same tier. A `spec:` locator, a `test:` locator outside `test_paths` and any locator with a `#scope` or an `@sha256` pin stay `Realized-by` and are not flagged. It offers no edit action, because the comment must exist before the locator goes (an exemption with a reason, as for the back-pointer rules). Opt-in, and needs `back_pointer_paths`; `test_paths` is optional and, as for the back-pointer rules, each entry must lie inside a `back_pointer_paths` entry. The population counts the declared `Realized-by` slots. The optional `scope: changed` option (`ADR-90`, `MILE-203`) makes it a nudge for the moment someone touches a record: it then judges only the records a `--base` diff adds or changes (not their dependents), none on a plain `check`, and the slots it did not judge are counted in the population's `deferred_by_scope`, with one `info` notice saying why. |
| `config.scope-matches-nothing` | Judges the run itself, not the corpus: whether the configuration's declared rules reached what they were handed, from the other rules' own executions (`MILE-106`). |
| `waiver.matches-nothing` | A waiver that does nothing is reported on the waiver record, naming the rule and scope it claimed (`MILE-168`). Four cases: the waiver is expired (its `Expires` has passed or is not a `YYYY-MM-DD` date, so it already covers nothing); it is not in effect (it lacks a field the suppression type's `effective_requires` names, or holds a blank, placeholder or pending value there; or, when the type declares `permanent`, it has no `Expires` and its rule has no `permanent` entry, its category is absent or outside the entry's vocabulary, or a category that requires a citation has none that resolves; either way it suppresses nothing and its finding is reported again, naming the unmet requirement); it names a rule this build does not ship; or it is active, its rule ran, and no finding of that rule falls in its scope. A waiver matching a finding is not reported, and an expired waiver is reported once, as expired; an ineffective one is reported once, as ineffective. The unmatched case is judged only on a whole-corpus run in which the target rule ran: under a path scope or `--base`, or with the target rule off, an absent finding proves nothing, so the waiver is counted as eligible and unexamined (never out of scope). The expired and unknown-rule cases need only the waiver record to have been examined. Opt-in. The population counts waiver records. |
<!-- rule-table:end -->

### Status computed from Done-means items (`ADR-96`, `MILE-214`)

For a type that declares `statuses.delivery.items` (`SPEC-3`; the grammar is `SPEC-6`'s), the caller
reads each record's declared section into its parsed items and `status.delivery-agreement` judges the
`Status` from the items alone. With `n` items, `r` resolved, `r_d` of them resolved at a
`delivered_tiers` tier, and the unresolved items being the non-waived, non-resolved ones:

| Items | Standing | Tier reported | Status it agrees with |
|---|---|---|---|
| `r_d >= 1` and no unresolved item, and no unfinished dependency | delivered | the lowest tier among the resolved items | the type's `set_status` |
| `r_d >= 1` and no unresolved item, and an unfinished dependency | partial (capped) | the lowest tier among the resolved items, with the capping dependency named | `delivery.items.partial_status` |
| `r_d >= 1` and some unresolved item | partial | the same, with the unresolved items named | `delivery.items.partial_status`; also correct under any progress status |
| `r_d = 0` and `r >= 1` | specified only | the lowest resolved tier | not forced either way, as a spec-only record is today |
| `r = 0` (including `n = 0` and every item waived) | none | the declared `embodiment.none` | a not-done status |

Waivers excuse an item, they do not deliver one. The rule's variants for such a type are the existing
`delivery-not-done` (delivered while not done), `delivery-without-evidence` (a `success` status with `r = 0`),
and two new ones: `delivery-partial-not-started` (partial while the status is a not-done status short of
`partial_status`; `urzua fix --apply` writes `partial_status`) and `delivery-partial-blocked` (the same on a
derived-blocked record whose `partial_status` is a `progress` status, where `fix` writes nothing because it
would contradict `sequence.status-agreement`, and the finding names why).

**Dependencies cap the status** (`ADR-96`, amendment of 2026-11-07). The computed status is the lower of the
item result and the dependency result. A record whose items all resolve but which has an unfinished
dependency over a pair `roadmap.pairs` declares (`ADR-83`, `ADR-76`) computes the partial standing and
`partial_status`, not delivered: it agrees with `partial_status`, and neither `DoneWithUnresolvedItems`
(no item is unresolved) nor `delivery-not-done` is reported for it. A dependency is unfinished exactly as
`sequence.status-agreement` reads it: its `Status` is not a value its own type declares `success`, so one in a
`failure` status, one with no `Status` and one whose type declares no `success` all cap. The cap only lowers
a delivered standing, and applies only where the type declares `delivery.items` and the repository declares a
sequencing pair, so a configuration without either is unchanged. Under `delivery-partial-not-started` the
message names the capping dependency, as `MILE-171 (InProgress)`; `fix` prints a notice naming it for a record
the cap holds below its items, and does not downgrade a `success` status the cap contradicts (the derived
blocked value, `sequence.status-agreement`, reports that status). `sequence.status-agreement` skips a blocked
record whose `partial_status` comes from the cap, with an info notice `all items proven ahead of dependency X`
(variant `status-blocked-capped`), so `partial_status` may stay one of the type's `progress` statuses; a record
in that status with an unresolved item is still reported.

**A record's kind sets which tiers deliver** (`ADR-96`, amendment of 2026-11-07). Where the type declares
`delivery.items.kind`, the record's own `Kind` value selects the tiers at which an item counts as delivered:
the kind's `delivered_tiers` replace the type's in the rule that at least one item must resolve at a delivered
tier, so a `decision` kind that counts `Specified` computes delivered when every item resolves at `Specified`
or higher, and `fix` writes `set_status` for it. A kind with `cited_records: success` also holds each locator
that resolves to a record to that record's status: it counts only while the status is one its own type declares
`success`; a Proposed ADR or a Draft spec does not, the item is unresolved, and its cause reads
`locator <path> is <ID> at status <status>, which its type does not declare a success status`. A locator that
is not a record is not judged by status. A record with no `Kind`, or a `build` kind that names no tiers, is
judged as before, and a type with no `kind` key is unchanged.

**`DoneWithUnresolvedItems`** (action variant `done-with-unresolved-items`) is a guaranteed error: a
`success` status while any item is unresolved, whatever `r` and `r_d` are. The message is
`<ID> is <Status> but Done means item <n> "<text>" is unresolved: <cause>` once per unresolved item, in
item order, joined by `; ` in one finding anchored on the first unresolved item's line, where `<cause>` is
`no evidence clause`, `locator <path> does not exist`, or
`locator <path> resolves only to <tier>, which is not in resolving_tiers` (a clause that is not entirely
locators is reported by `status.done-items-shape` and reads as unresolved). Its severity is a floor: a
repository's level for the rule applies to the rule's other variants and cannot lower this one, so
`check` exits 1 on it under every `fail_on` and every level, `info` included. It is also unwaivable: no
`WAIVER` record matches a finding of this variant, whatever its `Rule` and `Scope`, and the rule's
`permanent` categories are not consulted for it; the waiver still matches the rule's other variants, and
a waiver that matches nothing is reported by `waiver.matches-nothing` as before. Only evidence (a
resolving locator) or an inline item waiver clears it. `urzua rules` and `urzua explain --rule status.delivery-agreement` print the floor
(`severity_floor`) and `waivable: false` on the variant. The variant declares two actions, `edit-field`
on the status field with the computed value (`partial_status` when `r_d >= 1` and the record is not
derived-blocked, else the first declared status that is neither a `success`, `failure` nor `progress`
status) and `edit-section` on the Done means item (`done-with-unresolved-items-evidence`); a finding
carries one `suggested_action` (`ADR-73`), so it carries the first and the table lists both. `fix` never
downgrades a `success` status (`SPEC-8`).

**`status.done-items-shape`** reports each deviation from the grammar, anchored on its line, with an
`edit-section` action, and a `Kind` value outside the declared vocabulary (an error naming the allowed
kinds; the record is judged as one with no kind). **`status.waived-item-stale`** (a warning by default, which a dated `WAIVER` record
can waive) reports a waived item whose evidence clause now resolves, whose cited record does not exist,
whose `deferred-to` target (a category declared `expires: required`) is finished (its status in the target
type's `success` or `failure`), or whose expiry date has come (from that date on). Every waived item is
listed in the report's `notices` at `info`, one per item and a summary by category, never omitted.

Item locators are evidence for every reader of the evidence field: `embodiment.locator-exists` checks
them (variant `item-locator-defect`, an `edit-section` action on the item; an item that carries a valid
waiver is not examined, because the waiver holds while its evidence does not resolve), `embodiment.consistency`
and drift read them with the commit that last touched the clause's own line, and `fix --rebaseline` re-pins
them (`SPEC-8`). Every message about the items names the section by the type's declared `items.section`,
never a fixed word.

### Not yet built

Design language from this spec's original Phase B scope, with no shipped rule behind it yet:

- **Status enum validation, tolerant of trailing free-text annotation**, and validated against
  corpus *prose*, not only instantiated values: a value that no record currently holds may already
  be committed to by another document, and is real a release before anything is set to it. Nothing
  validates `Status` against an enum for any record type today (found live via MILE-71's red-team of
  the milestone `Status` field).
- **Role rules** — Author / Reviewers / Deciders per type and status, with self-acknowledgement
  detection: an author is not their own reviewer of record.
- **Required sections** per profile, generalized beyond `change_class` on revision entries (the one
  instance that's actually built).
- **Declared scope** (RFC-11 §6) and **boundaries** (RFC-11 §1) — neither is built; RFC-11's own
  open questions (nested-scope inheritance, glob-vs-subject width) are still unresolved.

Severity per rule comes from config (SPEC-3) in principle; in practice severity is still hardcoded
per rule at the source (`Error`, `Warning` or, at a rule set to `level: info`, `Info`), not yet read from any config field —
tracked as MILE-80, which also names the still-open question of a richer severity taxonomy versus a
separate log-verbosity axis. `status` reports what was found; whether that fails the build is the
caller's choice and lives in `blocking`.

A finding's `severity` is `"error"`, `"warning"` or `"info"` (`ADR-87`). `info` is opt-in: it appears
only for a rule the repository sets to `level: info` (`SPEC-3`), or for a `field_levels` entry or a
boundary `severity` of `info`. An info finding is a full finding (rule, file, line, message,
population and `suggested_action` as at `warning`), is listed in `findings`, and counts toward
`status`: a run with an unwaived info finding is `findings-present`. It never sets `blocking` and never
trips `fail_on` (see Exit codes). A rule set to `info` reports nothing above `info`: a `field_levels`
entry or a boundary `severity` declared higher does not lift it. A config that uses no `info` produces
a byte-identical report.

## Output contract

Implements RFC-3, as amended by ADR-7 (real field names, not RFC-3's illustrative casing) and
ADR-26/ADR-46 (stdout is the only stream; there is no stderr rendering at all). Stdout carries nothing
but the JSON object below, so a consumer never has to find it inside prose.

```json
{
  "status": "ok",
  "files_examined": 51,
  "records_read_by_any_rule": 51,
  "rules_executed": [
    {
      "rule": "header.required-fields",
      "population": { "unit": "field", "eligible": 1248, "examined": 1248, "out_of_scope": 0, "unreadable": 0 },
      "status": "ran"
    },
    {
      "rule": "claim.status-agreement",
      "status": "not-enabled"
    }
  ],
  "rules_selected": null,
  "scope": { "source": "tracked-sweep", "base": null, "record_types": ["adr", "bug", "rfc"], "include_untracked": false, "untracked_included": 0 },
  "blocking": false,
  "findings": [
    {
      "rule": "pointer.target-status",
      "severity": "warning",
      "file": "docs/adr/ADR-48-x.md",
      "line": null,
      "message": "Derives-from: ADR-29 resolves, but its Status is Superseded"
    }
  ],
  "notices": [
    { "severity": "warning", "subject": "identity.collision", "message": "..." }
  ]
}
```

(`rules_executed`/`findings` truncated above for length; a real run lists every declared rule and every
finding.)

`rules_executed` is the field that distinguishes this from every check in the source corpora --
`files_examined` alone answers "did it read anything," not "did it *check* anything," and a rule set
narrowed by a config error yields a confident zero over a swept corpus. Each entry's `status` is
`"ran"`, `"not-enabled"` or `"not-applicable"` (`RuleStatus`); a `"ran"` entry carries `population` (`eligible`/`examined`,
plus `out_of_scope`/`unreadable` -- `ADR-55`'s disclosure of what the rule was handed versus what it
judged), a `"not-enabled"` entry carries none. `"not-applicable"` is an enabled rule the run cannot supply
what it judges: the diff rules (`diff.source-requires-record`, `diff.source-and-records-mixed`) judge the
changed set, so on a run with no `--base` they report it, carry no population and count as not having
looked, so a run whose only enabled rules are diff rules is `"not-run"`, never `"ok"`. On a `--base` run
their findings are about changed source paths rather than records, so the diff scope does not filter them
out, and a path argument does not either. Their path options (`source_paths`, `application_paths`,
`exclude_paths`) are normalised like `published_paths`, and each entry is either a path prefix or a glob
(`MILE-158`, `SPEC-3`); `exclude_paths` removes matching paths from `source_paths` as well as from
`application_paths`, so a diff touching only excluded files is not a trigger; those paths stay in the
population as eligible and unexamined (not out of scope, which `config.scope-matches-nothing` would
report as an unreachable configuration); "a governed record changed" means a changed `.md`
file directly in a record type's `dir`, deleted ones included, and a governed record is never application
source. Their population is counted in `changed-path`.

**`diff.decider-required` and `--actors`** (`ADR-89` section 4, `MILE-200`). A third diff rule judges the
changed set against the top-level `authority` table (`SPEC-3`): a record moved to a status a gate names
(read against the merge base's copy, as `status.transition-allowed` is) or a changed gated path needs the
pull request's author, or a login whose approval is current, in the gate's decider group. Those identities
are input and never fetched (`check` does not use the network): `urzua check --base REF --actors FILE`
reads `{"schema": 1, "pr_author": LOGIN, "approvers": [LOGIN, ...]}`, which the gate action writes from the
host's API listing only approvals given on the head commit, so a stale approval does not count. A file
that cannot be read or has another shape is a could-not-run (exit 2); one that no rule reads (the rule off,
filtered out by `--rules`, or no `--base`) is a warning notice; combined with `--stdin-path` it is refused.
With no `--actors` file a gated change is reported as `info`, since a pre-commit run cannot know, and as an
error at the rule's level under `require_evidence: true`; on a run with no `--base` the rule is
`not-applicable`. **The rule proves authorship, not who decided:** with a single maintainer, and agents
acting under the maintainer's token, every pull request is authored by a decider and the rule cannot tell
an agent from the maintainer.

A rule that declares `scope: changed` (`SPEC-3`, "Judging only the changed records"; `ADR-90`) judges only
the records a `--base` diff adds or changes, and none on a plain `check`. Its population gains one additive
counter, `deferred_by_scope`: the candidates that were present, belonged to a record the diff did not
change (or there was no diff), and were not judged. They are eligible and not examined, like any other
unexamined candidate, but are counted apart from `out_of_scope`, which keeps its meaning (a configuration
that does not reach a record) and is what `config.scope-matches-nothing` reads. `deferred_by_scope` is
present only when non-zero, so a rule that does not declare the option reports exactly what it did before,
and a consumer that does not know the field ignores it. The unit is the population's own (the `field`
slot for `embodiment.locator-on-back-pointer-path`, not the record); a slot that is absent is counted as
absent, not deferred. The report carries one `info` notice per such rule, with the rule id as its subject,
naming `scope: changed`, the number of slots not judged and why (`no --base`, `unchanged since <base>`, or
`not the proposed record` for `--stdin-path`, which treats the proposed record as the one changed record).
Dependents are not changed: only the records the diff itself touches are judged. `waiver.matches-nothing`
leaves a waiver on such a rule eligible and unexamined. `urzua rules` lists each rule's `options` and a
`judges` value (`one-record`, `anchor-scoped`, `cross-record`, `changed-set`, `not-record-level`) saying whether it may
take the option. It also lists each rule's `scope`, its `population_unit` (and a `companion_population_unit` for a rule with a second population), and its `spec`; `urzua explain --rule <id>` (`SPEC-13`) prints one rule's entry with the level and option values this repository sets.

A rule has one population per entry, and a second unit is a second entry: `citation.resolution` with
`non_reference_field` set (`ADR-75` clause 3, `MILE-186`) appears twice in `rules_executed`, first
with its `citation` population and then, under the same rule id, with a `field-entry` population
(`PopulationUnit::FieldEntry`, serialized `field-entry`): one candidate per entry a record declares in
that field, whether valid, uncited or malformed. Every entry is examined and none is out of scope, so
`config.scope-matches-nothing` does not read it as a misconfiguration; a run where no record declares
anything reports `eligible: 0`, distinguishable from a rule that was not enabled, and a repository that
does not set the option sees one entry as before. The uncited and malformed findings are counted in the
`field-entry` population and not in the `citation` one; their rule id, variants and actions are unchanged.

`status` is the real `ReportStatus` enum: `"ok"`, `"findings-present"`, or `"not-run"` -- never `"ok"`
when zero files were selected, keyed on the file count, not on a missing scope, nor when no rule
looked at anything. The one exception is a `git-diff` run whose diff reaches no governed record
while some rule is enabled and ran: that is `"ok"` with `files_examined: 0` and an info notice. `notices` (`ADR-46`)
is non-fatal observations (`Info`/`Warning` only) that never affect `status` or the exit code, omitted
entirely when empty. When the suppression type declares `permanent` (`SPEC-3`), `check` always adds an info
notice with the subject `permanent-waivers` counting the permanent waivers in effect by category
(`ADR-86`). Two more info notices belong to discovery (`ADR-92`): `discovery.untracked-skipped`, above,
when the option is off, and `discovery.untracked-not-drift-judged` when it is on. A record with no git
history (an untracked file, a staged-new one, an unborn `HEAD`) has no date for its `Realized-by` line, so
`embodiment.consistency` cannot say that a cited file changed since: it does not judge the record for
drift, and the notice gives the count of such records that carry a locator, so the skip is disclosed and
not inferred. A record is judged by drift once it has a commit. The same is true of a `Finding`'s own `waived` field, omitted unless a waiver
covers it.

Every finding's `file` is relative to the repository root, including a finding about the configuration
(`.urzua/config.yaml`), so a report compares and baselines across checkouts (`BUG-185`). A `--config`
outside the repository has no relative form and is named as given.

### Suggested action

`ADR-73` decides this field; this section reproduces its vocabulary. A finding may carry one
`suggested_action`: the machine-actionable form of what to do about it, so an agent need not read
`message`. It is additive: a finding without one is unchanged, and `message` is always present.

```json
{
  "rule": "field.quality",
  "severity": "error",
  "file": "docs/bugs/BUG-9-x.md",
  "line": null,
  "message": "field 'Regression-test' is Blank -- not a real, present value",
  "suggested_action": {
    "kind": "edit-field",
    "op": "set",
    "target": { "file": "docs/bugs/BUG-9-x.md", "field": "Regression-test" }
  }
}
```

Keys are snake_case and kinds kebab-case, like the rest of the report. The vocabulary (a value in
`kind`, the `op` an edit takes, the `target` keys and the `value`):

| kind | required `target` keys | optional `target` keys | `op` (required) | `value` |
|---|---|---|---|---|
| `edit-field` | `file`, `field` | `line` | `set`, `remove`, `append`, `rename` | a string (a list-valued header uses the comma-separated form `SPEC-2` reads). For `set`, `append` and `rename` (the new name) only when derivable and single-valued; omitted means a human chooses. For `remove`, omitted removes the whole field and a value names the entry removed from a list field |
| `edit-section` | `file`, `section` | `line` | `add`, `set`, `remove` | never (prose) |
| `edit-config` | `file`, `key_path` (array of path segments, never a dotted string) | | `set`, `remove`, `append` | only when single-valued: the YAML value as JSON (a string, a list of strings, or a boolean). For `remove`, omitted removes the whole key and a value names the entry removed from a list |
| `add-record` | | `type` (omitted when not determinable), `file` (path to create, if known) | none | never |
| `run-command` | `argv` (array) | | none | never |

- `file` is repository-root-relative, as in findings (`SPEC-2`, `BUG-185`), and need not equal the finding's `file`: for the reciprocity rules the finding sits on the claimant and the edit belongs on the counterpart. `line` is given only where it differs from the finding's. `field` is the declared spelling.
- One flat key vocabulary: `file`, `field`, `section`, `type`, `rule`, `scope`, `key_path`, `argv`, `line`. A new key is added to this table, not invented per rule.
- `value` is any JSON value of the types in the table: a string, a list of strings, or a boolean; no other type occurs in the appendix. A key or `value` that was not computed is omitted, never `null`. The same holds for an absent `suggested_action`.
- `argv` is concrete in the emitted finding (no placeholders), `argv[0]` is `urzua`, and a consumer adds `--by` only when no identity resolves (`SPEC-8`).
- `run-command` means: run exactly this command; the command is the source of truth for what it writes. For `fix` and `index --apply` it is offered only when the command would produce that repair for this finding (a test checks each): `urzua fix --apply --ids <record>` for `embodiment.consistency` and `status.delivery-agreement`, `urzua index --apply` for `index.freshness`, `urzua refs refresh` for `citation.issue-exists`, which is not judged by that test: it needs the network and a token, and an unresolved citation may be a typo, so that command is a lead and `citation.issue-exists` declares `edit-section` set as its action instead. Because it carries no `value`, a `fix` repair stays the one place its computed value lives; `edit-field` carries a value only where no `fix` repair exists (relation reciprocity, `field.untrimmed-value`, `header.field-case-mismatch`, `config.pointer-declaration-missing`). "Derivable" follows `ADR-15`'s eligibility clauses. A test asserts that every finding offering `urzua fix` has a matching repair in `urzua fix` output for that record and field. A `run-command` can still be refused by the command's own gates (`--ids`, revision-log requirements); the finding promises the command, not its success.
- `waive` is not in the first release. A finding carries exactly one `suggested_action`, so there is no slot for a secondary action, and a kind with no producing rule would be a contract without a test. Adding `waive` (a waiver is a record of the `waiver` type, `ADR-11`) and a secondary-action mechanism such as an `alternatives` array are a later, separate decision; both are non-breaking under the compatibility rule.
- There is no `note` key: `message` is the prose and is always present.

Declaration. The declaration is a data table, not a function, in `RULE_TABLE` (`MILE-171`): each rule lists, in `actions`, one `VariantAction` per finding variant it can emit, naming the variant and an `ActionDecl`, which is either an action (a `kind` and, where the kind has several, an `op`) or `Exempt(&'static str)` with a reason, modelled on `HeaderNone::Exempt`. A rule never builds an action: it attaches an `ActionSource` to the finding, naming the variant and carrying only what it computed (a field name, a configuration key path, a value), and the engine joins that to the declaration. A variant with neither an action nor an exemption, a variant a rule emits that is not declared, and a declared variant no rule emits fail a coverage test; a source missing a key its declared kind requires asserts in a debug build. Exemption reasons are shown by `urzua rules`, so "no action by design" is discoverable although it is an omitted field on the wire. A waived finding carries its action unchanged.

Compatibility. The set of kinds, `op` values and `target` keys is closed per release and extensible across releases. Adding a kind, an `op` value or an optional `target` key is non-breaking. Renaming or removing one, changing its meaning, or changing which keys are required is breaking. Consumers MUST treat an unrecognised `kind`, or an unrecognised `op` on a known kind, as non-actionable and fall back to `message`, and MUST ignore unrecognised `target` keys. `SPEC-2` states this in its output contract. The report has no output version field today; if one is added it is a separate record and kind additions stay minor.

The action each finding variant declares, or the reason it declares none, is generated from the same
rule table as the table above (`urzua rules` lists it as each rule's `actions`; an exemption shows
its `exempt` reason), so this section and the rule table cannot disagree. Regenerate with
`make rule-table`.

When `config.pointer-field-not-known`, `config.relation-field-not-known`,
`relation.target-status-undeclared` or `config.companion-declaration-missing` names a type that declares
no `known_fields`, appending the one field would create a one-item list and switch
`header.field-set-consistency` on for every other optional field the type's records use. The
`-declare-set` variants therefore carry `edit-config` `set` with the full list: every field the type's
records use beyond its `required_fields`, plus the field in question, sorted. A type that already
declares `known_fields` keeps the `append` variant.

<!-- action-table:start -->
| Rule id | Variant | Declared action |
|---|---|---|
| `header.required-fields` | `header-region-missing` | exempt: the header region is neither a field nor a section, so no action kind addresses it |
| `header.required-fields` | `duplicate-key` | `edit-field` `remove` |
| `header.required-fields` | `field-missing` | `edit-field` `set` |
| `pointer.resolution` | `pointer-unresolved` | `edit-field` `set` |
| `pointer.resolution` | `pointer-untracked` | exempt: the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration |
| `pointer.target-status` | `pointer-target-status` | `edit-field` `set` |
| `field.quality` | `field-quality` | `edit-field` `set` |
| `field.pending` | `field-pending` | `edit-field` `set` |
| `claim.status-agreement` | `claim-status-disagrees` | exempt: the rule catches a claim that may be false, so the right fix is often the claim file, a non-record file with no action kind yet, and editing the record Status would be the mechanical answer and possibly the wrong one |
| `claim.status-agreement` | `claim-closes-open` | `edit-field` `set` |
| `claim.status-agreement` | `claim-id-unresolved` | exempt: the id names no record, so there is no record to edit; the fix is to correct the id in the claim file, a non-record file with no action kind yet, or to file the record |
| `claim.status-agreement` | `claim-field-malformed` | exempt: the edit is to the claim file, a non-record file with no action kind yet |
| `claim.status-agreement` | `claim-field-missing` | exempt: the edit is to the claim file, a non-record file with no action kind yet, and which records the change closes is the author's to say |
| `filename.title-consistency` | `title-mismatch` | exempt: the fix is a rename or an H1 edit, and no action kind addresses either yet |
| `relation.supersession-reciprocity` | `supersession-unresolved` | `edit-field` `set` |
| `relation.supersession-reciprocity` | `supersession-untracked` | exempt: the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration |
| `relation.supersession-reciprocity` | `supersession-unreciprocated` | `edit-field` `append` |
| `relation.pair-reciprocity` | `pair-unresolved` | `edit-field` `set` |
| `relation.pair-reciprocity` | `pair-untracked` | exempt: the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration |
| `relation.pair-reciprocity` | `pair-unreciprocated` | `edit-field` `append` |
| `relation.cross-type-disallowed` | `cross-type-claim` | `edit-field` `remove` |
| `sequence.cycle` | `sequence-cycle` | exempt: breaking a cycle means choosing which dependency to drop, a decision the tool cannot compute |
| `sequence.cycle` | `sequence-cycle-cross-pair` | exempt: breaking a loop that crosses declared roadmap pairs means choosing which dependency, in which pair, to drop, a decision the tool cannot compute |
| `sequence.unresolved-blocker` | `blocker-unresolved` | `edit-field` `set` |
| `sequence.unresolved-blocker` | `blocker-untracked` | exempt: the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration |
| `sequence.unresolved-blocker` | `blocker-not-a-reference` | `edit-field` `set` |
| `sequence.status-agreement` | `status-blocked-unblocked` | `edit-field` `set` |
| `sequence.status-agreement` | `status-blocked-derived` | `edit-field` `set` |
| `sequence.status-agreement` | `status-blocked-capped` | exempt: an info notice, not a defect: the record's `partial_status` comes from the dependency cap (every Done means item resolves, only an unfinished dependency holds it back), so there is nothing to repair |
| `relation.target-status-undeclared` | `target-status-undeclared` | `edit-config` `append` |
| `relation.target-status-undeclared` | `target-status-undeclared-declare-set` | `edit-config` `set` |
| `revision-log.change-class-required` | `change-class-missing` | `edit-section` `set` |
| `revision-log.change-class-required` | `revision-log-missing` | `edit-section` `add` |
| `revision-log.change-class-required` | `revision-log-column-unreadable` | `edit-section` `set` |
| `revision-log.version-bump-required` | `version-bump` | `edit-field` `set` |
| `embodiment.consistency` | `embodiment-drift` | `run-command` |
| `embodiment.consistency` | `embodiment-unevidenced` | `edit-field` `set` |
| `embodiment.consistency` | `last-audit-date-invalid` | `edit-field` `set` |
| `embodiment.consistency` | `claim-too-old` | exempt: the remedy is re-verifying the claim, which may need no edit, and touching the field to reset the age without checking would defeat the rule |
| `embodiment.claim-expired` | `claim-expired` | `run-command` |
| `embodiment.claim-expired` | `claim-anchor-invalid` | `edit-field` `set` |
| `embodiment.claim-renewal-unattested` | `claim-renewal-unattested` | exempt: the remedy is a judgement: undo the hand-typed date, or have someone re-verify the claim with `urzua reverify`, which writes the row |
| `embodiment.requires-accepted-status` | `embodiment-state-not-accepted` | `edit-field` `set` |
| `embodiment.back-pointer-resolution` | `back-pointer-unresolved` | exempt: the edit is to a source file comment, and no action kind addresses a non-record file yet |
| `embodiment.back-pointer-resolution` | `back-pointer-uncovered-path` | `edit-config` `set` |
| `embodiment.back-pointer-resolution` | `back-pointer-uncovered-test-path` | `edit-config` `set` |
| `embodiment.back-pointer-decision-accepted` | `decision-not-implementable` | exempt: the edit is to a source file comment or to the cited record, and no action kind addresses a non-record file yet |
| `embodiment.back-pointer-decision-accepted` | `decision-unjudgeable` | exempt: the edit is to a type or rule declaration that depends on which statuses permit implementation, a policy choice with no single target |
| `embodiment.back-pointer-decision-accepted` | `decision-uncovered-path` | `edit-config` `set` |
| `embodiment.locator-exists` | `locator-empty` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-missing` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-untracked` | exempt: the repair is `git add` on the named file, and no action kind addresses git: `run-command` is closed to `urzua` commands and `edit-config` edits configuration |
| `embodiment.locator-exists` | `locator-malformed` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-scope-unresolved` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-scope-unpinned` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-commit-anchor` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-kind-undeclared` | `edit-field` `set` |
| `embodiment.locator-exists` | `locator-kind-unpinnable` | `edit-field` `set` |
| `embodiment.locator-exists` | `item-locator-defect` | `edit-section` `set` |
| `embodiment.locator-promotion-candidate` | `locator-promotion-candidate` | exempt: an advisory nudge that is never auto-promoted |
| `header.field-set-consistency` | `undeclared-field` | `edit-field` `remove` |
| `header.field-set-consistency` | `undeclared-field-case` | `edit-field` `rename` |
| `field.untrimmed-value` | `untrimmed-value` | `edit-field` `set` |
| `field.leading-reserved-indicator` | `leading-reserved-indicator` | `edit-field` `set` |
| `header.field-case-mismatch` | `field-case-mismatch` | `edit-field` `rename` |
| `header.case-variant-key` | `case-variant-key` | exempt: the rule reads the keys a record wrote, not a declared vocabulary, so it cannot tell the declared spelling from the stray one and a remove could delete the declared key |
| `narrative-field.stale` | `narrative-field-stale` | `edit-field` `set` |
| `type.dir-matches-nothing` | `exclude-files-matches-nothing` | `edit-config` `set` |
| `type.dir-matches-nothing` | `extra-dir-missing` | `edit-config` `set` |
| `type.dir-matches-nothing` | `dir-missing` | `edit-config` `set` |
| `type.dir-matches-nothing` | `exempt-status-matches-nothing` | `edit-config` `set` |
| `identity.collision` | `identifier-collision` | exempt: the fix is a renumber, and no action kind addresses a rename yet (`urzua migrate ids` exists) |
| `identity.open-pr-collision` | `open-pr-collision` | exempt: the fix is to rename the file and its H1 to the next free number, or a waiver naming a closed pull request; neither is a field edit |
| `type.record-outside-declared-dir` | `record-outside-declared-dir` | exempt: the fix is a move, and no action kind addresses a move yet |
| `type.no-declared-spec` | `no-declared-spec` | exempt: a signal to review, never a mandate |
| `header.pointer-field-clean` | `pointer-field-not-clean` | `edit-field` `set` |
| `config.companion-declaration-missing` | `companion-field-missing` | `edit-config` `append` |
| `config.companion-declaration-missing` | `companion-pair-undeclared` | `edit-config` `append` |
| `config.companion-declaration-missing` | `companion-field-missing-declare-set` | `edit-config` `set` |
| `config.companion-declaration-missing` | `companion-pair-undeclared-declare-set` | `edit-config` `set` |
| `config.pointer-declaration-missing` | `pointer-declaration-missing` | `edit-config` `set` |
| `config.pointer-field-not-known` | `pointer-field-not-known` | `edit-config` `append` |
| `config.pointer-field-not-known` | `pointer-field-not-known-declare-set` | `edit-config` `set` |
| `config.pointer-narrative-overlap` | `pointer-narrative-overlap` | exempt: a field named in both lists could belong in either, and which list should keep it is the author's decision, not a computable repair |
| `config.header-none-has-no-required-fields` | `header-none-field` | `edit-config` `remove` |
| `config.relation-field-not-known` | `relation-field-not-known` | `edit-config` `append` |
| `config.relation-field-not-known` | `relation-field-not-known-declare-set` | `edit-config` `set` |
| `config.known-fields-declaration-missing` | `known-fields-declaration-missing` | `edit-config` `set` |
| `config.schema-version-outdated` | `schema-version-outdated` | `run-command` |
| `filename.identity-pattern` | `identity-pattern-none-match` | exempt: the fix is a rename, and no action kind addresses a rename yet |
| `filename.identity-pattern` | `identity-pattern-mismatch` | exempt: the fix is a rename, and no action kind addresses a rename yet |
| `status.undeclared-value` | `status-undeclared-value` | `edit-field` `set` |
| `status.required-field-missing` | `status-required-field-missing` | `edit-field` `set` |
| `status.forbidden-field-present` | `status-forbidden-field-present` | `edit-field` `remove` |
| `status.transition-allowed` | `transition-not-allowed` | `edit-field` `set` |
| `status.unreachable-state` | `status-unreachable` | `edit-config` `set` |
| `status.delivery-agreement` | `delivery-not-done` | `run-command` |
| `status.delivery-agreement` | `delivery-without-evidence` | exempt: no single correct value exists and `urzua fix` writes nothing for it; the repair is adding evidence, which may be a locator or a source back-pointer |
| `status.delivery-agreement` | `delivery-partial-not-started` | `run-command` |
| `status.delivery-agreement` | `delivery-partial-blocked` | exempt: the record is derived-blocked with an unresolved item, and `urzua fix` writes no `progress` `partial_status` to it because `sequence.status-agreement` would refuse it; the repair is finishing the open dependency |
| `status.delivery-agreement` | `done-with-unresolved-items` | `edit-field` `set`; severity floor `error`; not waivable by a rule-level waiver |
| `status.delivery-agreement` | `done-with-unresolved-items-evidence` | `edit-section` `set`; severity floor `error`; not waivable by a rule-level waiver |
| `status.done-items-shape` | `done-items-shape` | `edit-section` `set` |
| `status.waived-item-stale` | `waived-item-stale` | `edit-section` `set` |
| `date.field-valid` | `date-invalid` | `edit-field` `set` |
| `date.field-order` | `date-order` | `edit-field` `set` |
| `citation.resolution` | `citation-unresolved` | exempt: the fix edits prose located by line, and no action kind addresses a line in prose yet |
| `citation.resolution` | `non-reference-malformed` | `edit-field` `set` |
| `citation.resolution` | `non-reference-uncited` | `edit-field` `remove` |
| `citation.path-exists` | `path-missing` | `edit-section` `set` |
| `citation.issue-exists` | `issue-missing-in-section` | `edit-section` `set` |
| `citation.issue-exists` | `issue-missing-in-field` | `edit-field` `set` |
| `citation.issue-exists` | `snapshot-unusable` | exempt: regenerating the snapshot with `urzua refs refresh` needs the network and a token, so the command is named in the message and not offered as an action |
| `boundary.published-path-reference` | `published-path-reference` | exempt: the edit is to a published non-record file, and no action kind addresses a non-record file yet |
| `boundary.published-path-reference` | `published-path-uncovered` | `edit-config` `set` |
| `boundary.published-path-reference` | `generated-back-pointer` | exempt: the edit is to a generated non-record file, which is overwritten, and no action kind addresses a non-record file yet |
| `boundary.published-path-reference` | `boundary-uncovered` | `edit-config` `set` |
| `boundary.published-path-reference` | `boundary-exclude-unused` | `edit-config` `remove` |
| `boundary.published-path-reference` | `boundary-marker-unused` | exempt: the edit removes a marker from a non-record file, and no action kind addresses a non-record file yet |
| `section.required` | `section-missing` | `edit-section` `add` |
| `section.shape` | `section-shape` | `edit-section` `set` |
| `index.freshness` | `index-stale` | `run-command` |
| `index.freshness` | `index-unregenerable` | exempt: `urzua index --apply` cannot regenerate this artifact (not text, a missing marker or an unparsable format), so the artifact or the declaration is edited by hand |
| `diff.source-requires-record` | `source-without-record` | `add-record` |
| `diff.source-requires-record` | `source-without-decision` | `add-record` |
| `diff.source-and-records-mixed` | `source-and-records-mixed` | exempt: process advice, no file edit |
| `diff.decider-required` | `gated-move-without-decider` | exempt: accepting a record is a decider's act; no edit to the record supplies the pull request's author or approval |
| `diff.decider-required` | `gated-path-without-decider` | exempt: changing a protected path is a decider's act; no edit to the file supplies the pull request's author or approval |
| `override.unreviewed` | `override-unreviewed` | exempt: reviewing an override is a person's act, recorded in the log's `Reviewed-by` cell; no field of a record holds it |
| `override.unreviewed` | `override-log-missing` | exempt: the log is not a record: creating it writes a file, which no action kind addresses (the hook creates it on the first override) |
| `override.unreviewed` | `override-log-malformed` | exempt: the log is not a record: a malformed row is repaired by editing the log file, which no action kind addresses |
| `embodiment.locator-on-back-pointer-path` | `locator-on-back-pointer-path` | exempt: the repair is a comment in a source file, and no action kind addresses a non-record file yet; dropping the locator first would delete the evidence before the comment exists |
| `embodiment.locator-on-back-pointer-path` | `locator-on-back-pointer-path-sole` | exempt: the repair is a comment in a source file, and no action kind addresses a non-record file yet; dropping the only locator first would leave the record with no evidence |
| `config.scope-matches-nothing` | `scope-matches-nothing` | `edit-config` `set` |
| `waiver.matches-nothing` | `waiver-expired` | `edit-field` `set` |
| `waiver.matches-nothing` | `waiver-ineffective` | `edit-field` `set` |
| `waiver.matches-nothing` | `waiver-unknown-rule` | `edit-field` `set` |
| `waiver.matches-nothing` | `waiver-matches-no-finding` | `edit-field` `set` |
<!-- action-table:end -->

### Rule selection: `--rules` (`ADR-79`, `MILE-190`)

`urzua check --rules <id>[,<id>...]` restricts a run to the named rules of the repository's own
declaration. Selection narrows what runs and never enables anything (`ADR-53`).

- **`rules_selected`** is a top-level field of every report: the ids as given, in order and without
  repeats, or `null` on a run that was not restricted. It is always present, so a consumer never
  has to infer a filter from the length of `rules_executed`.
- **An excluded rule is omitted from `rules_executed`.** It is not listed as `not-enabled`, which would
  say the repository declined a policy it declared, and it is not run: a selection that leaves out
  `embodiment.consistency` skips the git history that rule reads, and one that leaves out the
  back-pointer and boundary rules does not read the source files under `back_pointer_paths` or the
  published boundary files. `status`, `blocking` and the exit code
  are computed over the rules that ran exactly as over a full run. A rule that is selected but
  declared `off` is listed `not-enabled`, so a selection of only such rules is `not-run` (exit 2), as a
  run with nothing enabled always is.
- **Rules that read other rules' results see the selection.** `waiver.matches-nothing` judges a live
  waiver only when its rule ran over the whole corpus, so under a selection that leaves the rule out the
  waiver is eligible and not examined, never reported unmatched.
- **A mistake in the command is exit 2, not a finding.** An empty id (`a,,b`), an id this build does not
  ship, and an id the configuration does not declare are each `not-run` with an `error` naming the id.
- Combines with a path argument, `--base` and `--stdin-path`; the selection applies to the rules, the
  scope to the records.
- `urzua audit` has no selection of its own (`SPEC-11`).

## The roadmap output (`urzua roadmap`)

`urzua roadmap` (`MILE-182`, `ADR-82`, `RFC-48`) is read-only and prints one `RoadmapReport`:
`{"status": "ok", "roadmap": {...}}`, exit 0 whenever the roadmap could be computed (a cycle or a
record to re-plan is data in the output, not a failure), exit 2 as for every command when the
configuration or corpus cannot be read. `export --format json` carries the same object under `roadmap`
when `roadmap.pairs` is declared, and omits the key otherwise (`SPEC-1`). Nothing is stored.

**One graph.** The edges are those of the pairs `roadmap.pairs` names (`SPEC-3`), read in either
field, so a written `Blocks` counts, through the graph builder `sequence.cycle` uses. A loop that
crosses two declared pairs is a cycle here, and `sequence.cycle` reports it too (`MILE-192`): with `roadmap.pairs` declared it reads those pairs as the same one graph, so `check` and the roadmap agree. A
claim on a record whose header cannot be read, or that resolves to nothing, draws no edge and is a
note. An edge `A Depends-on B` runs `B -> A`: blocker first.

| Key | Meaning |
|---|---|
| `schema` | Integer, `1`. Adding a key, a note kind or a `reason` does not change it; changing what a number means does. |
| `pairs` | The declared `roadmap.pairs`, in declaration order. |
| `waves` | Array of arrays of ids. A record's wave is `0` with no open dependency, else `1 +` the largest wave among its open dependencies; a dependency in a `success` status is ignored. Layer `n` holds wave `n`. |
| `ready` | Open records that are not derived-blocked and not kept out by `needs_replan`. `blocked` is derived only (`ADR-83`): a record is blocked when a direct dependency is not in a status its own type declares `success`; a written status is never read for it. With every type work, ready is wave 0 by construction, and both are emitted. With `roadmap.work_types` declared, `ready` is a subset of wave 0: a work record whose only open dependency is a non-work record is in wave 0 and `blocked`, and one that sits behind open work through a finished non-work record is unblocked and in a later wave. |
| `critical_path` | The longest chain of included records by count, and among the longest the lexicographically smallest id sequence, blocker first. Computed over the whole chain, not greedily. An empty list when no included record has an included dependency, since every record is then a chain of one and none is critical. |
| `records` | Map keyed by id, in natural order, of every open record, `needs_replan` ones included: `type`, `status` (the written value, `null` when none), `wave` (`null` when kept out), `ready`, `blocked`, `blocked_by` (direct dependencies not in `success`), `blocks` (every open record that transitively depends on this one, never itself). |
| `groups` | `roadmap.group_by` as nested arrays: `{field, value, groups}` for a non-final field, `{field, value, records}` for the final one. Values sort naturally, the `null` bucket (no value) last. Holds the same set as `records`. `[]` when `group_by` is empty. |
| `needs_replan` | `{record, reason, via}` per open record kept out of `waves`, `ready` and `critical_path`. |
| `waiting_on` | Present only when `roadmap.work_types` is declared: `{record, type, status, blocks}` per non-work record not in a `success` status (a `Dropped` one included, since someone must re-point its dependents), in natural id order. `blocks` lists the work records that transitively depend on it. An empty array when every non-work record is finished. |
| `cycles` | Each strongly connected component of more than one record, or a record with an edge to itself, once, members ascending. |
| `notes` | `{kind, record, pair, field, message}` from a closed set. |

**Items.** A record of a type that declares `statuses.delivery.items` (`SPEC-3`) also carries `items: {total, resolved, waived}`, so a reader sees "3 of 5" without a status the engine would have to know; no other record has the key. A partially delivered record is open work: it stays in its wave and in the ready set when nothing it depends on is open, and its dependents stay blocked, because blocked is derived from the written `Status` of the dependency, which `status.delivery-agreement` holds to the items.

**Open.** A record is open when its status is in neither `success` nor `failure` of its own type. A
record whose type declares no status vocabulary, or that states no `Status`, is open and draws a
`no-status-vocabulary` note. In scope are the records of types that declare a side of a declared
pair, and every record an edge reaches, whatever its type.

**Work types** (`MILE-196`, `ADR-82` Amendment of 2026-11-05). With `roadmap.work_types` absent every
type is work and nothing in this section changes, `waiting_on` included. With it declared, a record of
a type it does not list is non-work: it stays a node of the one graph and gates its dependents as any
record does (a dependent of an `Open` one is `blocked` and not `ready`, of one in `success` released,
of one in `failure` in `needs_replan` as `failed-dependency` with `via` the non-work record), but it is
in no wave, not in `ready` or `critical_path`, not a key of `records`, not in `groups`, not in
`needs_replan` as a record, not counted by `nothing-open` and not a subject of `unstructured-wait`;
`no-status-vocabulary` is still raised for it. Exclusion still propagates through it. Waves and the
critical path count work records only and contract their edges through chains of non-work records,
whatever those records' status: with work `W`, non-work `C` depending on `W`, and work `R` depending
on `C`, `W` is in wave 0, `R` in wave 1, and `W`, `R` is a chain of length 2. `blocks` of a work
record lists work records only and reaches them through non-work ones. `sequence.cycle` and
`sequence.status-agreement` never read `work_types`, so they give the same findings with and without
it, and `cycles` lists a loop's members whatever their type.

**Natural order.** Every id list, map key set and `waves` layer is ascending in natural id order: the
text before a trailing number compared as text, then the number as a number, then the whole as text,
so `MILE-5` precedes `MILE-12`. Group values sort the same way.

**`needs_replan.reason`** is a closed set. `failed-dependency`: a direct dependency is in a `failure`
status (`via` is that dependency, the smallest id if several). `cycle`: the record is a member of a
cycle (`via` is `null`). `depends-on-cycle`: a direct dependency is kept out because of a cycle
(`via` is the smallest such dependency). `depends-on-excluded`: a direct dependency is kept out for a
failure further down (`via` is the smallest such dependency, and the root cause is reachable by
following `via`). A record with several reasons takes the first of `cycle`, `failed-dependency`,
`depends-on-cycle`, `depends-on-excluded`. The way out of a failure is to re-point the `Depends-on`
(a `Superseded` target at its successor); the record then re-enters a wave. A cycle is reported for
every strongly connected component of the graph, including one whose other members are finished;
only its open members appear in `needs_replan`.

**`notes[].kind`** is a closed set, sorted in this order, then by `record` in natural order, then by
`pair` (`null` first), then `field`, then `message`:

| Kind | When |
|---|---|
| `no-sequencing-pair` | `roadmap.pairs` is empty. `records` is `{}` and every list is empty. The message says whether any pair declares `acyclic: true`. |
| `pair-matches-nothing` | A declared pair no record carries a value for. |
| `unresolved-claim` | An entry in a declared pair's field that names no record, names a record of a type the pair does not carry, or is prose. |
| `unreadable-record` | A record whose header cannot be read but whose type declares a pair side, or a claim on one. |
| `no-status-vocabulary` | An in-scope record whose type declares no status vocabulary or that states no `Status`. |
| `unstructured-wait` | An open record that states a wait in one of its type's `narrative_fields` (placeholders, pending values and `—` excluded) and carries no value in the forward field of a declared pair. Stays until the `condition` record type exists. |
| `group-field-unset` | A `group_by` field no open record carries. |
| `nothing-open` | No in-scope record is open: `waves` is `[]` because everything is finished, not because nothing was declared. |

A consumer treats an unknown `kind`, `reason` or key as informational, as for `kind` in `ADR-73`.
The command emits notes and data, never findings, so it has no suggested action; the rules that
judge the same records (`sequence.cycle`, `sequence.unresolved-blocker`) are unchanged.

## Exit codes

| Code | When |
|---|---|
| 0 | no findings at or above the configured blocking severity; an `info` finding never counts, under either `fail_on` |
| 1 | blocking findings; under `fail_on: warning` (or `--fail-on warning`), also any unwaived warning (`SPEC-3`) |
| 2 | the check could not run — bad config, unreadable path, explicit paths matching zero files |

`--fail-on error|warning` overrides the configuration's `fail_on` for one run. It moves only the exit
code: `blocking` and the rest of the report are unchanged.

2 is distinct on purpose. A tool that cannot run and a tool that ran and found problems are
different states, and collapsing them is how "0 errors" comes to mean "never executed".

## Testing

- **Every rule ships with a planted-violation test** (RFC-11 §5). An absence assertion carries no
  information until its failure has been observed; a rule with no such test is unverified, not
  passing.
- **Corpus tests run against SPEC-4's anonymized corpora**, not synthetic fixtures, for every
  bug in SPEC-1's acceptance suite.
- **The no-silent-no-op property is itself tested**: a run over an empty selection must produce
  `not-run`, and a run with a rule set of zero must not report `ok`.
- **A derived rule reports the evidence it found, not only the findings it emitted.**
  Files-examined and rules-executed are not sufficient: four documented derivations each returned a
  clean, plausible zero, and **three of them would have passed a
  files-and-rules report** — the records were examined and the rules did run. What was empty was the
  evidence the rule had to reason over. A history-reading rule that resolved no history for any
  record reported a fully clean corpus; the cause was a `git log` invocation that stops at a rename,
  in a corpus that had just renamed its record homes.

  So a rule whose findings depend on derived input reports that input's size:
  `transition: 91 records, 0 with readable history` is self-refuting on its face, and `0 findings`
  is not.

  The defect is `eligible: N, examined: 0` — the rule was handed candidates and reached a verdict on
  none of them. That is §5 of RFC-11 applied to a rule's input rather than its output. `eligible: 0`
  is a *different* state and not a defect: it is the cold start of enabling a rule before its scope
  exists, which any rule declared before its scope exists occupies. Neither is a
  `check` verdict — what gets checked is the adopter's declaration (`ADR-53`) — and the separation
  between disclosing the population and judging it is `MILE-106`'s.
- **Fixture containment.** The rule tests deliberately contain record-shaped strings, and a
  self-referential fixture that the checker then scans is a documented trap. Fixtures live outside
  the discovered record set, and that exclusion is asserted rather than assumed.

## Out of scope

- `--fix` in any form. See RFC-8; needs the revision log first.
- **Writing.** `audit` was a deprecated alias of `check` (ADR-79, `SPEC-11`); both commands are read-only. The data-loss incident this
  spec originally cited was about a bulk cross-reference *rewrite*, not about reading reciprocity —
  `check` running the same read-only rule `audit` does was never actually the risk.
- Judging whether a section's *content* covers the scope it claims. SPEC-1 records this as a
  permanent ceiling on mechanical checking, not a maturity gap. `check` is never sold as replacing a
  human reader.

## References

- SPEC-1 — v0 CLI: success criteria, acceptance suite, the permanent ceiling.
- SPEC-3 — configuration, which supplies record types, severities and boundaries.
- SPEC-4 — the corpora this is tested against.
- RFC-3 (output), RFC-10 (header), RFC-11 (boundaries and scope), RFC-13 (status-scoped
  checks), ADR-3 (identity).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Bumped to `0.10` (`BUG-21`). §Output contract's JSON block rewritten to the real shipped shape: `snake_case` fields, the real 3-value `ReportStatus`, `rules_executed` as the real per-rule array (not a count) with its `population`/`status` shape, `records_read_by_any_rule`, `notices`, and a `Finding`'s real fields (no `suggestedAction`, which never existed). The 2026-09-11 entry below named this drift and filed it rather than fixing it; this revision is that fix. | **substantive** |
> | 2026-09-21 | Criterion 2 no longer says "in both output formats", and re-keyed the zero-population rule in §Acceptance. **Why:** two claims that had gone untrue. There is one output format -- `ADR-23` and `ADR-26` removed the second -- so the criterion was unsatisfiable as written and no amount of correct implementation could have met it. And "a rule reporting a zero population is `not-run`" was written when a rule reported one number; now that a population carries `eligible` and `examined` separately, the two zeros mean opposite things, and treating `eligible: 0` as a fault makes enabling a rule before its scope exists fail the build (`BUG-84`). The criterion also now requires the unit, because a count without one held three different denominators (`BUG-40`). | **substantive** |
> | 2026-08-20 | Split out of SPEC-1, which retains the cross-cutting rules. | **structural** |
> | 2026-09-07 | Added the Embodiment consistency, drift, and locator-promotion rules (ADR-18/ADR-32) to the rule set — previously implemented but never listed here. Noted that the rest of this section's rule names predate and don't match the actual shipped rule ids, as a named gap rather than silently compounding it. | **substantive** |
> | 2026-09-20 | Criterion 1 and Phase A now say `urzua check`, not `urzua check docs/`. **Why:** the scope argument selects which findings are reported, so naming `docs/` excluded every finding about a file outside it -- `claim.status-agreement` reports on the claim file in `.changeset/`, and the gate therefore could never fail on a false claim, which is the one thing that rule exists to catch (`BUG-86`). The criterion's intent is that the tool checks this repository and fails on a broken record; `docs/` was the whole corpus when it was written and no longer is. | **substantive** |
> | 2026-09-09 | Rewrote §Rules for MILE-90/ADR-44: `pointer.resolution`/`header.pointer-field-clean` are now config-driven (`pointer_fields`/`narrative_fields` per type, no hardcoded field list); `blocked-on.stale` renamed `narrative-field.stale`, generalized beyond `Blocked-on`; added the three new config-level rules (`config.pointer-declaration-missing`, `config.pointer-field-not-known`, `config.pointer-narrative-overlap`) and the two rows this table had never listed at all (`type.no-declared-spec`, `header.deprecated-shape`) despite both already shipping. Corrected the stale "ten rules" framing to the real, current seventeen. **Why:** this table drifting behind the actual shipped rule set is the exact recurring gap this spec's own 2026-09-07 entry already named once; MILE-90 touched every one of these rule functions directly, making this the natural point to close the gap rather than let it recur a third time. | **structural** |
> | 2026-09-07 | Rewrote §Rules as a complete, accurate list of all ten actually-shipped rule ids (adding `header.layout-consistency`/ADR-38 and `header.field-set-consistency`/ADR-39, neither previously mentioned at all), separated from the design language that's still unbuilt. Corrected §Out of scope's claim that cross-record reconciliation is `audit`-exclusive -- `check` has run `relation.supersession-reciprocity` directly since ADR-30, and the actual data-loss risk this spec was guarding against was a bulk *rewrite*, never a read-only reciprocity check. **Why:** per ADR-14's amendment adopted earlier the same day, a spec's body must stay a complete, replayable specification of its subject at every revision, not accumulate "not yet reconciled" notes as a substitute for actually updating it -- leaving this stale on the very day that policy was adopted would have been an immediate, visible contradiction. | **substantive** |
> | 2026-09-07 | Added `blocked-on.stale` as an 11th rule (ADR-42); `pointer.resolution`'s row updated to include `Parent`/`Blocked-on`, which it had already gained (ADR-40) without this table being updated. **Why:** the same "spec must stay complete" policy applies to every rule addition, not just the ones made on the day the policy was adopted -- letting this table go one rule stale again immediately would have repeated the exact drift this spec was just corrected for. | **substantive** |
> | 2026-09-08 | Reworded the header-format-consistency seed rule's rationale (§"The two seed rules") from a present-tense claim about the corpus's current state to historical framing. **Why:** ADR-33/ADR-42's corpus-wide `yaml-frontmatter` migration made the original wording -- "this corpus of 22 records carries three header formats" -- false the moment it landed; caught by adversarial review before the migration shipped, rather than left as another stale-prose instance for a future pass to find. | **substantive** |
> | 2026-09-08 | Bumped to `0.5`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Noted `notices` as a real, shipped addition to the output shape (ADR-46), and named -- rather than silently compounded -- the pre-existing drift between §Output contract's illustrative RFC-3-era JSON block and the real shipped shape (camelCase vs. `snake_case`, stderr rendering that no longer exists, a `status` enum that doesn't match `ReportStatus`). Filed as its own bug rather than fixed in this revision. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | `Status: Draft` → `Accepted`, with `Embodiment`/`Realized-by` declared. **Why:** `check` is the most-built command in the tool and has its own integration-test file. This spec reached `Version: 0.8` -- eight revisions -- while still marked as a draft of something unbuilt. Verified against the binary before flipping rather than flipped in bulk -- `BUG-26` asked for exactly that, and it is why `SPEC-4` and `SPEC-5` are not flipped with these. | **substantive** |
> | 2026-09-19 | Rule count corrected from seventeen to twenty-one, and `ALL_RULES` named as the source this list must match. **Why:** `MILE-80` and the fixes after it added `pointer.target-status`, `field.pending`, `claim.status-agreement` and `embodiment.locator-exists` without updating the spec that calls itself *"the complete, current set"* -- so `check`'s own spec understated what `check` runs by four. Nothing checks the two agree, which is filed separately. | **substantive** |
> | 2026-09-19 | The four rules shipped since `MILE-80` added to the table: `pointer.target-status`, `field.pending`, `claim.status-agreement`, `embodiment.locator-exists`. **Why:** the count was corrected to twenty-one in the same change that left the list at seventeen rows, so the section contradicted itself in adjacent lines and the rule this repository declares `error` was undocumented in the spec calling itself complete. `BUG-52` is the missing mechanism. | **substantive** |
> | 2026-09-19 | `embodiment.locator-exists`'s row states the on-disk requirement. **Why:** it read *"absent from the git-tracked set"*, which was the rule's first form and silent on a staged deletion -- `git rm` drops a path from `ls-files` while leaving it in `diff --cached`. The spec described a check the code no longer performs. | **substantive** |
> | 2026-09-23 | Rule count corrected from twenty-one to twenty-nine; the eight rows `ALL_RULES` had gained since without this table being updated added (`type.dir-matches-nothing`, `type.record-outside-declared-dir`, `identity.collision`, `config.scope-matches-nothing`, `field.untrimmed-value`, `header.field-case-mismatch`, `config.header-none-has-no-required-fields`, `config.relation-field-not-known`). **Why:** found by a `RFC-42`/`ADR-61` review, which noticed this list's own self-declared invariant ("the complete, current set... `ALL_RULES` is the source it must match") had gone untrue by seven rows before its own change added an eighth. `BUG-52` remains open as the actual mechanism gap — nothing enforces this agreement automatically, so this is a manual sync, not a fix. | **substantive** |
> | 2026-09-23 | `argv` marked produced, scoped to a directly-named file; the Discovery prose states the directory-argument exception explicitly. **Why:** `BUG-24` fixed the gap this table had marked unbuilt -- an explicit argv path naming one file now overrides the tracked-only filter, while a directory argument still respects "never a raw directory walk" (`ADR-6`), which the table's blanket "explicit paths ... override discovery" line did not previously distinguish. | **substantive** |
> | 2026-09-23 | The rule table is now generated, not hand-maintained: `urzua_core::rules::RULE_METADATA` is the single source, `scripts/generate-rule-table.py` renders it between `<!-- rule-table:start/end -->` markers, `make rule-table-check` (wired into `make ci`) fails the build on drift. Backfilled the two rows `ALL_RULES` had gained since the last manual sync (`config.known-fields-declaration-missing`, `relation.target-status-undeclared`) and corrected `config.header-none-has-no-required-fields`'s description for its round-22 broadening. **Why:** `BUG-52` named this as the real fix -- not a comparison rule re-deriving the same fact a second time, but collapsing to one source so there is nothing left to drift. `BUG-52` is `Fixed`. | **substantive** |
> | 2026-11-02 | `pointer.target-status` gains the optional `fields` option, restricting the rule to named declared pointer/narrative fields; the rule's row in the generated table is updated. **Why:** one `not_in` list judged lineage (`Derives-from`) and dependency (`Implements`) fields alike, so a Superseded lineage target warned as stale (`BUG-172`). Omitted, the rule is unchanged. | **substantive** |
> | 2026-11-03 | `header.case-variant-key` added to the generated rule table: an opt-in, declaration-free rule reporting two keys in one record's header that differ only by case. **Why:** `MILE-113` -- a type declaring no vocabulary had nothing that flagged a `Status`/`status` pair, and `ADR-57` rules out making `duplicate_keys()` case-insensitive, so the signal is a separate rule that changes no exact-match verdict. | **substantive** |
> | 2026-11-03 | Adds `boundary.published-path-reference` to the rule table, with its required `published_paths` option, a list of path prefixes (never globs; a glob is treated literally), normalised at config load (leading `./`, trailing `/` and whitespace removed; an absolute entry, a `..` component or a backslash is a config error). An identifier split across lines is not detected. **Why:** `RFC-11` (decided in `ADR-9`) declares a set of paths that publish beyond the repository, and a record identifier cited from inside one is a violation (`MILE-125`). The rule's `level` is the boundary's severity; a declared path covering no tracked file is itself a finding (a prohibition fails closed), and a file that is not text is counted unreadable, never clean. | **substantive** |
> | 2026-11-04 | Criterion 5 now reads "one adopter's codebase" and "that adopter's hand-written linter" in place of "one source codebase"; criterion 6 is marked as part of the Phase 1 exit, so "all five hold" is now the full Phase 0 set (`SPEC-1`, Exit bar); `Version` `0.11` -> `0.12`. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | Bumped to `0.14`. `relation.pair-reciprocity` and `relation.cross-type-disallowed` added to the generated rule table; both require declared `relation_pairs` (`SPEC-3`) and `relation.supersession-reciprocity` is unchanged beside them. **Why:** `MILE-122` -- reciprocity over any declared pair of inverse fields, including across record types (`RFC-47`). | **substantive** |
> | 2026-11-04 | `citation.resolution` added to the generated rule table: an opt-in rule reporting a decision ID cited in a record's prose that resolves to no record, with a population counted in citations (new `citation` population unit). **Why:** `MILE-136` -- `pointer.resolution` reads only declared header fields, so a dangling prose citation passed every rule. | **substantive** |
> | 2026-11-04 | Bumped to `0.17`. `index.freshness` added to the generated rule table: an opt-in rule comparing a committed index of one record type (a whole file, or a list between declared `begin`/`end` marker lines in a README) with the records, in a declared line `format` with `{id}`, `{file}` and `{title}` placeholders. It reports a record with no entry, an entry naming no record, a duplicated entry, a stale file name or title, a line the format does not match, a missing artifact and a missing marker pair; it never regenerates anything and does not compare order. Its required `indexes` option (`path`, `type`, `format`, optional `begin`/`end`) is validated at config load, and its population counts the artifacts compared. **Why:** `MILE-139` -- a repository that commits an index had nothing that stopped it going stale. | **substantive** |
> | 2026-11-03 | Bumped to `0.13`. `git-diff` marked produced: `urzua check --base <ref>` reports only on records changed since the merge base of the ref and `HEAD` (working tree included), `scope` gains an always-present `base` (`null` unless `git-diff`), and §Discovery states the mode's contract, including that it is refused alongside an untracked named file. **Why:** `MILE-123` -- a pull-request gate over a large corpus should not sweep every record, and the table had reserved the value while marking it unbuilt so no consumer would branch on it before a run could produce it. | **substantive** |
> | 2026-11-04 | `git-diff` scope examines the changed records **plus their dependents**, not the changed files alone: unchanged records that name a changed or deleted record's id, share a changed id, cite changed or deleted `Realized-by` evidence, or share a normalised `Realized-by` locator with a changed or deleted record. A configuration change puts every record in scope; an empty diff is `ok` with an info notice rather than `not-run`; `scope` gains `changed_paths`, `dependents` and `whole_corpus_reason` on `git-diff` runs; a ref with no merge base names that in its error. `Version` `0.13` stays (the `--base` row above is the same unreleased change). **Why:** review of `MILE-123` reproduced a hole -- deleting a target left its referrer unexamined, so `check --base` reported nothing where a full check reported `pointer.resolution` -- and an empty diff that could not be told from a clean pass of the whole corpus. | **substantive** |
> | 2026-11-04 | `git-diff` scope also treats as dependents the records covered by a changed or deleted waiver's old and new `Scope`; splits header values on commas when finding references; keeps `type.record-outside-declared-dir` findings for changed files; treats a symlinked configuration as changed by either path; an empty diff with no rule enabled is `not-run`. §Output `status` states the empty git-diff exception. **Why:** late review of `MILE-123` reproduced each as a case where `check --base` hid or mis-stated what a full check reports. | **substantive** |
> | 2026-11-04 | Bumped to `0.18`. `section.required` and `section.shape` added to the generated rule table: opt-in rules reading a type's declared `sections.required` and `exempt_statuses` (`SPEC-3`), with a population counted in `(record, required section)` slots (new `section` population unit). **Why:** `MILE-4`, `MILE-5`, `MILE-7` and `MILE-121` -- SPEC-2 named required sections per type as a rule category and nothing built it. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.16`. Adds `diff.source-requires-record` and `diff.source-and-records-mixed` to the generated rule table, a new `not-applicable` `RuleStatus` and `changed-path` population unit, and the `source_paths`, `application_paths` and `exclude_paths` rule options (`SPEC-3`'s path-option normalisation). **Why:** `MILE-124` and `MILE-142` -- two diff checks an adopter's hand-written linter makes, source change without a record change and a change mixing records with application source, need a declared home; they run only on `--base` and say so otherwise rather than passing. | **substantive** |
> | 2026-11-04 | Bumped to `0.15`. `header.field-set-consistency` gains the optional `allow_extension_prefix` option, off unless declared `true`: a header key beginning `x-` in any case is exempt from the rule, and an unprefixed undeclared key is still a finding. The option is declared per rule, not per type, so a type's `known_fields` still lists every field the rule judges by default; `pointer.resolution` and `narrative-field.stale` read only declared pointer and narrative fields, so a prefixed key is never one of them unless declared. The 'Not yet built' entry for the `x-` escape hatch is removed, and the rule-table row is regenerated from `RULE_METADATA`. **Why:** `MILE-141` -- the adopter's linter admits `x-` keys through its closed key set, so without an equivalent a repository either declares every local extension in config or loses typo detection on real keys. | **substantive** |
> | 2026-11-04 | `filename.identity-pattern` added to the generated rule table: an opt-in rule reporting a file directly in a type's `dir` whose name yields no identity under the type's declared `identity.pattern` (or the `TYPE-NNNN-slug.md` convention when none is declared). When every file of a type is off-pattern it reports once on the directory with each file's expected name; `audit` runs it too. **Why:** `MILE-120` -- such a file was a loaded record every identity-reading rule skipped in silence, and `RFC-46` was rejected, so renaming is the supported route. | **substantive** |
> | 2026-11-04 | `status.undeclared-value`, `status.required-field-missing` and `status.forbidden-field-present` added to the generated rule table: opt-in rules reading a type's declared `statuses` (`SPEC-3`). `field.pending`, `narrative-field.stale` and `claim.status-agreement` now read a declared type's own pending vocabulary and success/failure terminals in place of the compiled-in token and the rule's hand-copied status list; a type declaring nothing is judged as before. **Why:** `MILE-132` Phase 0, closing `BUG-65` and `BUG-178`. | **substantive** |
> | 2026-11-04 | Bumped to `0.19`. `embodiment.back-pointer-resolution` added to the generated rule table, with its required `back_pointer_paths` option (path prefixes, normalised at config load like `published_paths`): an `Implements: <id>` source comment is code-tier evidence for the cited record in `embodiment.consistency` and `urzua fix`, and a back-pointer citing no record is reported. Evidence is collected only while the rule is enabled; its findings are kept under `check <path>` scoping (`RULES_REPORTING_OUTSIDE_THE_CORPUS`). **Why:** `MILE-128` -- a first adopter relies on such comments, so the evidence model needed a place for them (`ADR-66`). | **substantive** |
> | 2026-11-04 | Bumped to `0.20`. `embodiment.consistency` gains three opt-in refinements, off by default: a `@sha256:` pin on a `Realized-by` locator replaces the git-history heuristic for that locator, `drift_grace_days` holds back drift newer than the window, and `claim_max_age_days` warns on an old claim; the generated rule table row carries them. **Why:** `MILE-129` -- unconditional drift had no sanctioned way to say a change was re-verified or not yet due. | **substantive** |
> | 2026-11-04 | Bumped to `0.24`. `embodiment.requires-accepted-status` added to the generated rule table: an opt-in rule reporting a record whose `Status` is not an accepted status but whose stated `Embodiment` is `Implemented` or `Verified`. Accepted means the type's declared `statuses.success`; a type declaring none uses the rule's optional `accepted_statuses` option, and with neither it is not examined. Its population counts the records the rule could judge. **Why:** `MILE-138` -- `embodiment.consistency` never read `Status`, so a proposed record could claim to be built and pass. | **substantive** |
> | 2026-11-04 | Bumped to `0.23`. `date.field-valid` and `date.acceptance-by-status` added to the generated rule table: opt-in rules that validate declared date fields as real `YYYY-MM-DD` calendar dates and require the acceptance date exactly on accepted-type statuses, with the population counting the date values or records examined. **Why:** `MILE-137` Phase 0: a replacement linter must reject `2026-13-45` and an acceptance date on a record nobody accepted. | **substantive** |
> | 2026-11-04 | Bumped to `0.21`. `embodiment.back-pointer-decision-accepted` added to the generated rule table: an opt-in rule reporting a source back-pointer that cites a record whose `Status` is not an accepted one, where accepted is the cited type's declared `statuses.success` or, for a type declaring none, the rule's optional `accepted_statuses`; with neither the status cannot be judged and that is reported. It takes the same `back_pointer_paths`, reads back-pointers exactly as `embodiment.back-pointer-resolution` does, and on a `--base` run judges only the files the diff changed. Its findings are kept under `check <path>` scoping. **Why:** `MILE-133` -- implementation citing a decision must not land ahead of it. | **substantive** |
> | 2026-11-04 | Bumped to `0.22`. `field.quality`, `field.pending` and `status.required-field-missing` gain the optional `field_levels` option: a finding for a listed field has that field's `warn` or `error` level, any other finding has the rule's `level`, and `check` keeps applying the rule level to every rule that declares no per-field level. **Why:** `MILE-140` -- one rule needs two severities for two fields. Off unless declared; `SPEC-3` states the option. | **substantive** |
> | 2026-11-04 | Bumped to `0.24`. `date.acceptance-by-status` removed from the generated rule table. **Why:** maintainer decision of 2026-11-04 (`MILE-137`): the acceptance date by status is expressed with `statuses.require` / `statuses.forbid` and `date.field-valid`, so one mechanism owns status-dependent fields. | **substantive** |
> | 2026-11-04 | Bumped to `0.25`. `embodiment.back-pointer-decision-accepted` now judges a back-pointer against the cited type's declared `statuses.implementable` rather than its terminal `success`; a type declaring none uses the rule's `accepted_statuses`, and with neither the status is reported as unjudgeable. Rule id unchanged. **Why:** the engine holds no type knowledge, so a terminal success status cannot stand for "implementation allowed" (`MILE-133`, `SPEC-3` 0.13). | **substantive** |
> | 2026-11-04 | Bumped to `0.26`. `embodiment.requires-accepted-status` also allows the claim at the type's declared `statuses.claim_retained`, not only `success`. Rule id unchanged. **Why:** maintainer decision of 2026-11-04, `MILE-138`: a record may keep an Implemented or Verified claim in a status reached only after acceptance. | **substantive** |
> | 2026-11-04 | Bumped to `0.27`. Inside `drift_grace_days`, `embodiment.consistency` accepts either the tier value or a stated `Drift detected` (the window only holds back reporting, so it never contradicts a record that already says so) and `urzua fix` leaves such a record alone; drift outside the window still computes `Drift detected`. The generated rule table row says so. **Why:** `BUG-179`. | **substantive** |
> | 2026-11-04 | Bumped to `0.28`. Exit code 1 also covers an unwaived warning when `fail_on: warning` or `--fail-on warning` is in force; the report itself is unchanged. **Why:** `MILE-145` -- a repository at zero warnings needs warnings to fail its own build. | **substantive** |
> | 2026-11-04 | Bumped to `0.29`. `citation.resolution` takes an optional `external_ids` option (exact ids or inclusive numeric ranges under a declared prefix); a matching citation is counted out of scope and not resolved. The rule table is regenerated. **Why:** `MILE-156` -- a public standard number sharing a record prefix (`RFC-4122`) was reported as dangling. | **substantive** |
> | 2026-11-04 | Bumped to `0.30`. `status.required-field-missing` also counts a whole value of the type's declared `statuses.pending` as missing. **Why:** `MILE-153` -- the declared placeholder vocabulary was read by `field.pending` only, so an accepted record reading `Unknown` satisfied a requirement. | **substantive** |
> | 2026-11-04 | Bumped to `0.31`. Documented that a file a type lists in `exclude_files` is dropped at discovery, and that every finding's `file`, a config-level one included, is repository-relative. `filename.identity-pattern`'s generated row says its census omits an excluded file. **Why:** `MILE-150`, `BUG-183` and `BUG-185`. | **substantive** |
> | 2026-11-04 | Bumped to `0.32`. `relation.pair-reciprocity` reports an unechoed claim from a claimant at a pair's `provisional.statuses` at the provisional level (default `warn`) and any other claimant at the rule's level. **Why:** `MILE-154` -- one severity for a Proposed record's claim and a broken accepted claim was either too loud or too quiet. | **substantive** |
> | 2026-11-04 | Bumped to `0.33`. The back-pointer matcher reads a full id list and trailing markers: `Implements: ADR-1 (note), ADR-2` yields both ids, a marker after code on the same line (`//`, `#`, `--`, `/*`, `<!--`) is read, and reading still stops at a segment that does not open with an id (`ADR-1 and ADR-2` yields `ADR-1`). `embodiment.back-pointer-decision-accepted` judges the same widened set. **Why:** `MILE-147` -- a matcher that stopped at the first non-id token or ignored a same-line marker dropped citations an author wrote, so a record lost code evidence it had. | **substantive** |
> | 2026-11-04 | Bumped to `0.34`. `embodiment.back-pointer-resolution` gains the optional `test_paths` option and reads `Verifies:` in those files as test-tier evidence; `embodiment.consistency` computes `Verified` from it, judges a record with no `Realized-by` field that a back-pointer cites (the finding names the evidence), and `urzua fix` repairs the same records. Rule table regenerated. **Why:** `MILE-146` and `MILE-149` -- evidence is read from the artifact instead of restated beside it. | **substantive** |
> | 2026-11-04 | Bumped to `0.35`. `relation.pair-reciprocity` excludes a claim to a record type outside a pair's declared `types` as out of scope. **Why:** `MILE-155` -- a pair between one type's records erred on a claim to a type that cannot hold the inverse field. | **substantive** |
> | 2026-11-04 | Bumped to `0.36`. `embodiment.consistency` gains the optional `sticky_states` option: a record stating a listed `Embodiment` value is examined and not reported as drift or as an old claim, and `urzua fix` leaves it alone. Rule table regenerated. **Why:** `MILE-148` -- `Deprecated` and `Inactive` are states a human sets and nothing can compute, so a finding the author cannot resolve by changing the artifact only trains readers to ignore the rule. | **substantive** |
> | 2026-11-04 | Bumped to `0.37`. The diff rules' path options accept globs, and `diff.source-requires-record` also takes `exclude_paths`; an excluded changed path is eligible and unexamined. **Why:** `MILE-158` -- prefix-only options could not say "every source file except tests", so a test-only change tripped the gate. | **substantive** |
> | 2026-11-04 | Bumped to `0.38`. Added the opt-in `citation.path-exists` rule: a backtick-quoted repository path in a declared reference section must exist in the working tree (population unit: the cited path). The rule table is regenerated. **Why:** `MILE-143` -- `embodiment.locator-exists` covers only `Realized-by`, so a stale path cited in References passed every rule. Cited issue and pull request numbers remain unbuilt. | **substantive** |
> | 2026-11-04 | Bumped to `0.39`. Added `date.field-order` to the generated rule table: an opt-in rule over a `date_order` entry (`Earlier <= Later`). **Why:** `MILE-157` -- cross-field date ordering, with well-formedness left to `date.field-valid`. | **substantive** |
> | 2026-11-04 | Bumped to `0.40`. `index.freshness` reads multi-line entries (a `format` of several lines), accepts a `format` with `{file}` and no `{id}`, and `{title}` now excludes the record's own id as well as a leading number; the rule table is regenerated. A new `urzua index [--apply]` command regenerates the declared indexes. **Why:** `MILE-152` -- index checking read one line per entry and required a visible id, and nothing regenerated an index. | **substantive** |
> | 2026-11-04 | Bumped to `0.41`. `type.dir-matches-nothing` also reports a type's `exclude_files` entry that names no tracked file directly in its `dir`. Rule table regenerated. **Why:** `MILE-150` -- a stale exclusion is a declaration that silently does nothing. | **substantive** |
> | 2026-11-04 | Bumped to `0.42`. `relation.pair-reciprocity` counts a claim to a record type outside a pair's declared `types` as eligible and unexamined instead of out of scope, so `config.scope-matches-nothing` no longer warns on it. **Why:** `MILE-155` -- the claim is legitimate; the configuration is not what failed to reach it. | **substantive** |
> | 2026-11-04 | Bumped to `0.43`. `citation.path-exists` takes an optional `skip_tables` option (a table whose rows are history is not read), and `citation.resolution` counts a declared external id as eligible and unexamined rather than out of scope; the rule table is regenerated, and the `citation.resolution` row no longer writes its numeric-range example as a citable id. **Why:** `MILE-143` -- revision-log rows name paths as they were; `BUG-186` -- a declared external id was reported as a misconfiguration. | **substantive** |
> | 2026-11-04 | Bumped to `0.44`. Removed the `header.layout-consistency` and `header.deprecated-shape` rules from the generated rule table: with the `blockquote` and `bold-list` shapes gone there is no layout to declare and no deprecated shape a config can hold. A record still written in a removed form is reported by `header.required-fields` with the form named, and naming either removed rule under `rules:` is an unknown-rule load error. **Why:** `MILE-85`. | **substantive** |
> | 2026-11-04 | Bumped to `0.45`. Added `waiver.matches-nothing` to the generated rule table: an opt-in rule reporting an expired waiver, a waiver naming an unknown rule, and an active waiver whose rule ran over the whole corpus and found nothing in its scope; a path-scoped or `--base` run does not judge a live waiver. **Why:** `MILE-168` -- a stale waiver stayed on file silently, the no-silent-no-op principle applied to the waiver record. | **substantive** |
> | 2026-11-04 | Bumped to `0.46`. `embodiment.consistency` reports a shallow clone and failing git calls as `embodiment-drift-git` notices instead of reading them as no drift. **Why:** `MILE-159` -- a shallow clone once produced a false zero. | **substantive** |
> | 2026-11-04 | Bumped to `0.47`. `revision-log.change-class-required` finds the log by the type's declared `revision_log` identity (default the line `**Revision log**`), not by any line containing the text; `citation.path-exists` takes `skip_revision_log` in place of `skip_tables`. Rule table regenerated. **Why:** `MILE-167` -- prose mentioning the literal made the rule read the wrong table. | **substantive** |
> | 2026-11-04 | Bumped to `0.48`. `section.required` and `section.shape` count a record exempted by `exempt_statuses` as eligible and unexamined instead of out of scope; the rule table is regenerated. **Why:** `BUG-187` -- the exemption is declared, and `config.scope-matches-nothing` warned about it as a misconfiguration. | **substantive** |
> | 2026-11-04 | Bumped to `0.49`. `embodiment.back-pointer-resolution`'s `test_paths` accepts globs as well as prefixes, and a glob matching no scanned file is a finding. **Why:** `MILE-160` -- test files named by suffix cannot be named by a prefix list. | **substantive** |
> | 2026-11-04 | Bumped to `0.50`. `embodiment.consistency` judges a record stating `Verified` or `Implemented` with no `Realized-by` field and no citing back-pointer, reporting it against the computed `Not started`; a record stating no tier stays unexamined. **Why:** `MILE-161` -- a claim with no evidence passed silently. | **substantive** |
> | 2026-11-04 | Bumped to `0.51`. `embodiment.consistency` takes the optional `drift_audit_field`: the `drift_grace_days` window counts from that declared last-audit date on a record when it has one, else from the cited file's last change, and a malformed date is a warning. **Why:** `MILE-166` -- a file changes for reasons unrelated to its evidence. | **substantive** |
> | 2026-11-04 | Bumped to `0.52`. `field.pending` and `status.required-field-missing` also read the pattern entries of `statuses.pending`; the rule table is regenerated. **Why:** `MILE-162` -- a placeholder family is declared once. | **substantive** |
> | 2026-11-04 | Bumped to `0.53`. `relation.pair-reciprocity` does not report a provisional claim whose peer names the claimant in the pair's declared `provisional.acknowledged_by` field; the rule table is regenerated. **Why:** `MILE-163`. | **substantive** |
> | 2026-11-04 | Bumped to `0.54`. `index.freshness` also compares declared-field placeholders (`{Status}`, `{Date}`) with the record's header and reports a differing or missing value. Rule table regenerated. **Why:** `MILE-164`. | **substantive** |
> | 2026-11-04 | Bumped to `0.55`. `section.required` and `section.shape` follow a required section's declared `aliases` and read a marked `y-statement` from the preamble when `preamble: true`; `check` adds an info notice per alias in use. Rule table regenerated. **Why:** `MILE-165`. | **substantive** |
> | 2026-11-04 | Bumped to `0.56`. `citation.resolution` takes the optional `skip_revision_log` option; the rule table is regenerated. **Why:** `MILE-156` -- append-only revision-log history cites ids as they were. | **substantive** |
> | 2026-11-04 | Bumped to `0.57`. `field.pending` gains the optional `pending_until_required` option, off unless declared `true`; the rule-table row is regenerated from `RULE_METADATA`. **Why:** `MILE-169`; `SPEC-3` states the option. | **substantive** |
> | 2026-11-04 | Bumped to `0.58`. Added the opt-in `citation.issue-exists` rule: an issue or pull request cited in a declared reference section (`#123`, or a link to the declared repository) must be in the committed snapshot written by `urzua refs refresh`; an absent, unreadable or foreign snapshot is one finding on the snapshot path and every citation is counted eligible and unexamined; every finding names the snapshot's generated date and `check` adds an info notice with it. `check` reads the file and never the network. The rule table is regenerated and the snapshot finding is exempt from path scoping. **Why:** `MILE-143` -- the issue and pull request half. | **substantive** |
> | 2026-11-04 | Bumped to `0.59`. Added the opt-in `status.delivery-agreement` rule to the generated rule table: a `Status` held to the delivery evidence the type declares in `statuses.delivery`, both ways. **Why:** `MILE-183`. | **substantive** |
> | 2026-11-04 | Bumped to `0.60`. `citation.issue-exists` also reads the header fields named by the optional `issue_refs.fields` (a finding names the field and its line); the rule-table row is regenerated. Off unless declared. **Why:** `MILE-184` -- numbers in header fields such as a BUG's `Found-in` were unchecked. | **substantive** |
> | 2026-11-04 | Bumped to `0.61`. The shipped rule set is `RULE_TABLE`, one entry per rule carrying its description and properties; `RULE_METADATA`, `RULES_REPORTING_OUTSIDE_THE_CORPUS`, `IDENTITY_DEPENDENT_RULES` and `RULES_REQUIRING_RELATION_PAIRS` are derived from it. No rule or output changes. **Why:** `MILE-171`: a rule's properties lived in parallel lists. | **structural** |
> | 2026-11-04 | Bumped to `0.62`. The rule table, `urzua rules` and `rules_executed` list rules in one order: the order `check` runs them, with `config.scope-matches-nothing` and `waiver.matches-nothing` last. `check` output is unchanged. **Why:** `MILE-171` step 2: `check` now runs the table, so the table's order is its execution order. | **structural** |
> | 2026-11-04 | Bumped to `0.63`. A path argument and `--base` keep or drop a finding by the anchor its rule's table entry declares (a record, the configuration, a file outside the corpus, a tracked path, a type's directory, or a record plus a file the rule's options declare), replacing the rule ids `check` named. No finding is kept or dropped differently. **Why:** `MILE-171` step 3: each new rule that reported on a non-record path had to be added to a list by hand or be silently dropped from a narrowed run. | **structural** |
> | 2026-11-04 | Bumped to `0.64`. `config.relation-field-not-known` also checks the `date` and `author` fields a type declares under `relation_fields` (`MILE-176`): one not in the type's `required_fields` or `known_fields` is a finding, as an undeclared relation role already was. **Why:** `urzua new` fills those fields by name, so an undeclared one was written to a header the rest of the engine never examines. | **substantive** |
> | 2026-11-04 | Bumped to `0.65`. A type's records are the files directly in its `dir` and each `extra_dirs` entry; `type.dir-matches-nothing` also reports an `extra_dirs` entry that does not exist. Rule table regenerated. **Why:** `MILE-130` -- one identity namespace across several directories. | **substantive** |
> | 2026-11-04 | Bumped to `0.66`. `section.required` and `section.shape` read a required section declared at a deeper `level`, optionally `under` a named parent, so a `###` heading is visible to them; the rule table is regenerated. **Why:** `MILE-98`. | **substantive** |
> | 2026-11-04 | Bumped to `0.67`. Added `config.companion-declaration-missing` to the generated rule table: a type declaring one field of a declared `companion_fields` pair without the other is reported. **Why:** `BUG-41` -- a type declaring `Embodiment` without `Realized-by` let records claim a state `embodiment.consistency` could not check. | **substantive** |
> | 2026-11-04 | Bumped to `0.68`. The rule table gains `status.transition-allowed`, which compares a changed record's `Status` at the merge base with the change under check and is not-applicable without `--base`, and `status.unreachable-state`. **Why:** `MILE-144`, `ADR-72`. | **substantive** |
> | 2026-11-04 | Bumped to `0.69`. `citation.resolution` takes an optional `non_reference_field` option: a declared id in the named header field is not resolved or reported in that record, a declared id never cited and a malformed entry are findings, and `check` prints one aggregated Info notice for declared ids that now match a real record. The rule table is regenerated. **Why:** `BUG-190`, decided by `ADR-75`. | **substantive** |
> | 2026-11-04 | Bumped to `0.70`. A finding may carry `suggested_action` (`ADR-73`, `MILE-44`): a `kind` from a vocabulary closed per release, an `op` where the kind has several, a flat `target` and a `value` only where one is derivable, each omitted rather than `null` when not computed. The output contract reproduces `ADR-73`'s per-kind table and compatibility rule; each rule's table entry declares an action or a reasoned exemption for every finding variant it emits, `urzua rules` lists them as `actions`, and the generated action table below the contract is read from the same entries. **Why:** `MILE-44` -- a finding said what was wrong and never what to do; additive to `check` JSON. | **substantive** |
> | 2026-11-04 | Bumped to `0.71`. `field.quality` gains the optional `template_placeholders` option, off unless declared `true`; the rule-table row is regenerated from `RULE_METADATA`. **Why:** `BUG-22`; `SPEC-3` states the option. | **substantive** |
> | 2026-11-04 | Bumped to `0.72`. `sequence.cycle` and `sequence.unresolved-blocker` added to the generated rule table; both read only `relation_pairs` entries that set `acyclic: true` (`SPEC-3`), and enabling either with no such pair is a config load error. **Why:** `MILE-27` -- the sequencing relation `RFC-48` section 1 proposes. | **substantive** |
> | 2026-11-04 | Bumped to `0.73`. Added `revision-log.version-bump-required` to the rule table (regenerated from `RULE_METADATA`): a diff rule that compares a changed record's revision log and version field against the merge base's copy, read by the CLI and handed to the rules as data. **Why:** `BUG-51`; `SPEC-3` states the declaration. | **substantive** |
> | 2026-11-04 | Bumped to `0.74`. `embodiment.locator-exists` also reports a locator that does not parse, a scope that does not resolve in the file (with the reason), a scope given without a pin and a commit anchor (not yet supported); the rule table is regenerated. **Why:** `MILE-185` / `ADR-74` -- a scope that stops resolving must be reported, never passed; `SPEC-3` lists every reason. | **substantive** |
> | 2026-11-05 | Bumped to `0.75`. `embodiment.consistency` judges a scoped pin by the hash of its resolved span: an edit elsewhere in the file is not drift, there is no file-based grace window for it, and a scope that does not resolve is reported by `embodiment.locator-exists` and not also as drift. **Why:** `MILE-185` / `ADR-74`. | **substantive** |
> | 2026-11-05 | Bumped to `0.76`. Realized-by names the implementing code and tests, ready for `Shipped` (`MILE-7`). **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-cli/tests/check_integration.rs, test:rust/crates/urzua-cli/tests/fail_on_integration.rs` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Bumped to `0.77`. Status changed from Accepted to Shipped. **Why:** Maintainer decision, 2026-11-05: its Realized-by evidence computes Verified and an independent review judged it sound. | **substantive** |
> | 2026-11-05 | Bumped to `0.78`. `status.transition-allowed` matches a record added at a path the merge base lacks to a record the diff deleted by `Stable-Id`, so a renamed record is judged against the status it had before the rename instead of reading as new; one with no `Stable-Id` or no match still reads as new, and one matching several deleted records is counted unreadable. The rule-table row is regenerated. **Why:** `BUG-191` -- renaming an Accepted ADR gave a false error, and renaming it while resetting its status to an initial one evaded the rule. | **substantive** |
> | 2026-11-05 | Bumped to `0.79`. `waiver.matches-nothing` gains the finding variant `waiver-ineffective` (`edit-field` `set` on the first missing field), reported on a waiver that lacks a field its type's `effective_requires` names; the waiver suppresses nothing. **Why:** `BUG-177`, maintainer decision of 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.80`. `revision-log.change-class-required` reports a record of a type that declares `revision_log: {required: true}` and has no revision-log section, with the new variant `revision-log-missing` offering `edit-section` `add`; a type that does not declare it leaves such a record unexamined. The rule-table and action-table rows are regenerated. **Why:** `BUG-50` -- absence of the log was indistinguishable from compliance, and SPEC-1 lost 18 revision rows without a finding. | **substantive** |
> | 2026-11-05 | Bumped to `0.81`. `filename.title-consistency` reads an H1's id as a filename's is read: the declared `identity` prefix and a number, or with no pattern a leading `PREFIX-N` or bare `N`; a title that matches neither is reported as carrying no record id, and an overflowing number as too large to compare. The rule-table row is regenerated from `RULE_METADATA`. **Why:** `BUG-32`, maintainer decision 2026-11-05; `# 2026-09-16 Notes` no longer reads as 20260916. | **substantive** |
> | 2026-11-05 | Bumped to `0.82`. Removed the no-git directory-walk fallback from Discovery: outside a git repository every command that reads one exits 2 with a "needs a git repository" error, as the binary already did. **Why:** `BUG-170`, maintainer decision 2026-11-05; the spec promised behavior that never existed. | **substantive** |
> | 2026-11-05 | Bumped to `0.83`. A reference to a record, or a `Realized-by` locator, whose file exists on disk but is not tracked by git is its own finding variant (`pointer-untracked`, `supersession-untracked`, `pair-untracked`, `blocker-untracked`, `locator-untracked`), distinct from the `-unresolved` and `locator-missing` variants for a target that exists nowhere. Each declares an `ADR-73` action: the missing variants keep `edit-field` `set`; the untracked ones are exempt, since the repair is `git add` and no action kind addresses git. The action table is regenerated. **Why:** `BUG-13`, maintainer decision 2026-11-05; the on-disk fact is supplied by the CLI so the rule layer stays pure. | **substantive** |
> | 2026-11-05 | Bumped to `0.84`. A `Realized-by` comma separates entries only when a category prefix follows it; otherwise it belongs to the locator path. **Why:** `BUG-129`, maintainer decision 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.85`. `check --stdin-path <path>` judges one record's proposed content against every enabled rule that can judge one record, without writing, with `scope.source` `stdin`, a `not-applicable` entry for each enabled rule that cannot report on a record, and findings kept for that record alone; `status.transition-allowed` and `revision-log.version-bump-required` are judged against HEAD's copy of the record. **Why:** `MILE-191`, `ADR-78` item 2 -- the editor hook judges every rule on the edited record, not only the computed `Embodiment`. | **substantive** |
> | 2026-11-05 | Bumped to `0.86`. `boundary.published-path-reference` reads declared boundaries (`SPEC-3`, "Boundaries"): a boundary's severity is the level of its findings and the rule's `level` only enables it; a back-pointer inside a `generated` boundary is a finding; an `exclude` entry, a boundary path or an ignore-marker use that matches or suppresses nothing is a finding; `published_paths` is read as a published boundary and now takes globs. Four finding variants are added to the action table, and `published-path-uncovered` keeps its valueless `edit-config` `set` targeting the entry, and `boundary-uncovered` offers the same. The rule-table rows are regenerated. **Why:** `MILE-125`, `ADR-80`. | **substantive** |
> | 2026-11-05 | Bumped to `0.87`. `override.unreviewed` added to the generated rule table and action table: an opt-in warning for a row of the committed override log (`override_log`, `SPEC-3`) whose `Reviewed-by` cell is blank, with a finding for a row that is not an entry and a population that counts log entries (`entry`). **Why:** `MILE-188`, `ADR-78` item 1 -- an editor-hook override must surface in the pull request that carries it. | **substantive** |
> | 2026-11-05 | Bumped to `0.88`. `embodiment.locator-on-back-pointer-path` added to the generated rule and action tables: an opt-in warning for a `code:` or `test:` `Realized-by` locator on a file whose `Implements:` or `Verifies:` comment would be read automatically, exempt from an edit action: removing the locator before the comment exists would delete the evidence, and the comment is in a source file. **Why:** `MILE-189`, `ADR-78` item 4 (`RFC-7` section 5, prefer the lighter mechanism). | **substantive** |
> | 2026-11-05 | Bumped to `0.89`. `check --rules <id>[,<id>...]` restricts a run to named declared rules, and every report gains a top-level `rules_selected` (the ids, or `null`); an excluded rule is omitted from `rules_executed` and not run, and an undeclared, unknown or empty id is a could-not-run (exit 2). `audit` is documented as a deprecated alias with no selection of its own. **Why:** `MILE-190`, `ADR-79`. | **substantive** |
> | 2026-11-05 | Bumped to `0.90`. Added `config.schema-version-outdated` to the generated rule table and the suggested-action table (`run-command` `urzua migrate config`): a config read by upgrading it from the previous schema version is one warning on the config file. A config outside the supported window is a could-not-run that also carries a `schema_version_error` object (`SPEC-3`'s "Supported schema versions"). **Why:** `ADR-77` / `MILE-34`. | **substantive** |
> | 2026-11-05 | Bumped to `0.91`. Output-contract change: a new `field-entry` population unit, and `citation.resolution` with `non_reference_field` set appears twice in `rules_executed` (its `citation` population, then a `field-entry` population counting each declared entry); the uncited and malformed findings move from the first to the second. A repository that does not set the option sees no change; a consumer must not key `rules_executed` by rule id, since the same id now appears twice (a map keeps the last entry, the `field-entry` one). The rule table is regenerated. **Why:** `MILE-186` / `ADR-75` clause 3 -- a declared-ids check judged inside the citation count could not be told apart from a run over no declarations. | **substantive** |
> | 2026-11-05 | Bumped to `0.92`. The rule table gains `sequence.status-agreement`, which holds a record's written `Status` to the derived `blocked` value (`statuses.progress`, `statuses.blocked`, `roadmap.pairs`; `SPEC-3`), with the finding variants `status-blocked-derived` and `status-blocked-unblocked` (`edit-field` `set` on `Status`, the first carrying the declared blocked status only when it is derivable). A record in a `progress` status is counted unreadable, not examined and clean, when its dependency does not resolve or cannot be read, also for a type that declares no `blocked` status. **Why:** `MILE-144`, decided in `ADR-83`. | **substantive** |
> | 2026-11-05 | Bumped to `0.93`. Adds the roadmap output (`urzua roadmap`): the `RoadmapReport` object, how waves, ready, the critical path, cycles and `needs_replan` are read, natural id order, and the closed set of note kinds. `critical_path` is an empty list when no open record has an included dependency, not one arbitrary record. **Why:** `MILE-182`, decided in `ADR-82`; maintainer decision of 2026-11-05 on the flat-repository case. | **substantive** |
> | 2026-11-05 | Bumped to `0.94`. `embodiment.locator-exists` takes the options `locator_kinds` and `pinnable` (default `[file]`): a directory or glob locator is legal when the repository declares that kind and it covers a tracked file; a kind not declared, and a pin on a kind not pinnable, are reported with the new variants `locator-kind-undeclared` and `locator-kind-unpinnable` (`edit-field` `set`); the rule table is regenerated. **Why:** `BUG-176` -- maintainer decision, 2026-11-05: which locator kinds are legal, and which can be pinned, is configuration; `SPEC-3` states the semantics. | **substantive** |
> | 2026-11-05 | Bumped to `0.95`. `diff.source-requires-record` takes the optional `decision_types`, `fix_types` and `link_fields` options and, with `decision_types` declared, is the decision gate: the diff must change a decision record at a decided status, a record linking one, or a fix record, and the finding offers the new `source-without-decision` variant (`add-record`, naming the type when one type holds decisions). Without them the rule is unchanged. **Why:** `MILE-133`, maintainer decision 2026-11-05: extend the existing diff rule instead of adding a parallel one. | **substantive** |
> | 2026-11-05 | Bumped to `0.96`. `claim.status-agreement` reads a fragment's `closes` frontmatter key when `SPEC-3` declares it: a listed id that resolves to no record (`claim.id-unresolved`), a listed record not at a terminal status its type declares (an error naming the record, offering an `edit-field` on its `Status`), a value that is not a one-line list of record ids (`claim.field-malformed`) and, under `strict`, a fragment with a frontmatter block and without the key (`claim.field-missing`) are findings; a file with no frontmatter is counted out of scope and named in a notice (under `strict`, an unterminated `---` fence or a blank line before the opening fence is `claim.field-malformed` instead, because the release tool rejects both), and `check` prints how many closing-verb claims the deprecated prose reading still found. Four finding variants are added to the action table and the rule table is regenerated. **Why:** `BUG-66`, decided by `ADR-85`. | **substantive** |
> | 2026-11-05 | Bumped to `0.97`. The `waiver-ineffective` message names whichever requirement is unmet instead of always naming `effective_requires`: a missing field, a rule the type's `permanent` declaration does not list, a category outside its vocabulary, or a missing or unresolved citation (`SPEC-3`, `SPEC-10`). `check` adds an info notice, `permanent-waivers`, counting permanent waivers by category when `permanent` is declared. The `waiver.matches-nothing` rule-table row is regenerated. **Why:** `MILE-193` / `ADR-86`. | **substantive** |
> | 2026-11-05 | Bumped to `0.97`. A finding's `severity` gains `"info"` (`ADR-87`, `MILE-194`): reported, listed in `findings` and counted in `status` like a warning, never `blocking` and never an exit-1 under either `fail_on`; a rule set to `info` caps every finding at `info`. §Rules and Exit codes say so. **Why:** `MILE-194` builds `ADR-87`. | **substantive** |
> | 2026-11-05 | Bumped to `0.98`. The `waiver-ineffective` message names whichever requirement is unmet instead of always naming `effective_requires`: a missing field, a rule the type's `permanent` declaration does not list, a category outside its vocabulary, or a missing or unresolved citation (`SPEC-3`, `SPEC-10`). `check` adds an info notice, `permanent-waivers`, counting permanent waivers by category when `permanent` is declared. The `waiver.matches-nothing` rule-table row is regenerated. **Why:** `MILE-193` / `ADR-86`. | **substantive** |
> | 2026-11-05 | Bumped to `0.99`. `status.delivery-agreement`'s generated row now says the rule applies to any type that declares `statuses.delivery`, that `back_pointer_evidence: false` stops a back-pointer counting as evidence, and that `unevidenced_tiers` may be empty so a type never reports the second direction; the finding for a not-done record names the evidence field it was computed from. **Why:** `ADR-84`, accepted by the maintainer on 2026-11-05. | **substantive** |
> | 2026-11-05 | Bumped to `0.100`. `sequence.cycle` reads the pairs `roadmap.pairs` names as one graph, so a loop that uses an edge of each of two declared pairs is reported on every member, with the cycle's path and the pairs it crosses in the message, under the new finding variant `sequence-cycle-cross-pair` (exempt: which dependency to drop is a choice). A loop inside one declared pair keeps its message and variant; an `acyclic` pair not named in `roadmap.pairs`, and a repository with no `roadmap.pairs`, are read per pair as before. **Why:** `MILE-192`, following `ADR-82`: `check` and `urzua roadmap` now agree on a cross-pair loop. | **substantive** |
> | 2026-11-05 | Bumped to `0.101`. A YAML frontmatter header keeps each number and boolean as written (`Version: 1.10` reads `1.10`, not `1.1`), skips a leading byte-order mark instead of reading the header as absent, and keeps a non-string key (`1: x`) under its text rather than dropping it; `citation.issue-exists` refuses a refs snapshot whose `issues` or `pulls` are not strictly ascending. **Why:** Phase 0 hardening of the core model: each one made a rule judge a value or file other than the one on disk. | **substantive** |
> | 2026-11-05 | Bumped to `0.102`. `check --base` pulls in an unchanged record whose `Realized-by` glob locator covers a changed file (it had compared the glob as a path prefix and missed it); a missing merge base in a shallow clone says so and how to fix it instead of "the histories are unrelated"; `status.transition-allowed` counts a base copy it cannot read as unreadable, with a notice, instead of as a new record (rule-table row regenerated). The `back-pointer-uncovered-path`, `back-pointer-uncovered-test-path` and `decision-uncovered-path` edit-config actions now target the configuration file instead of the declared source path. **Why:** review of the CLI and the core rules found each: `check --base` exited 0 where a full check exits 1, a read error skipped a transition check, and an agent applying the action edited a source path. | **substantive** |
> | 2026-11-05 | Bumped to `0.103`. The generated description of `embodiment.locator-on-back-pointer-path` states that a record of a type declaring `statuses.delivery.back_pointer_evidence: false` is not flagged and that a locator whose file already carries the comment is told to drop the locator. **Why:** hardening review: the nudge told a `Fixed` bug to drop its only counted evidence, and told a locator to add a comment that was already there. | **substantive** |
> | 2026-11-05 | Bumped to `0.104`. A path or diff scope computes embodiment drift and claim ages for the records in scope only, since a record's drift reaches no other record's finding; `filename.identity-pattern`'s all-files-mismatch finding names the first five renames and counts the rest. **Why:** Phase 0 hardening from an adopter switchover -- `check <one path>` took as long as the whole corpus, and the finding named 466 renames in one string. | **substantive** |
> | 2026-11-05 | Bumped to `0.105`. Adds the roadmap output of `roadmap.work_types` (`MILE-196`): a non-work record gates its dependents but is in no wave, `ready`, `critical_path`, `records`, `groups` or `needs_replan`; waves and the critical path contract through non-work records; `ready` is a subset of wave 0; a top-level `waiting_on` lists the non-work records not finished. Absent, the output is unchanged. **Why:** `ADR-82`'s Amendment of 2026-11-05, accepted by the maintainer. | **substantive** |
> | 2026-11-05 | Bumped to `0.106`. The `claim.status-agreement` rule description and notice cover `verb_claims` (`MILE-197`): with it `false` no prose is read for a claim and `check` states that the reading is off instead of counting, and adds a notice when `strict` is off. **Why:** `ADR-88`, accepted by the maintainer. | **substantive** |
> | 2026-11-05 | Bumped to `0.107`. A population gains the additive `deferred_by_scope` counter (present only when non-zero) for a rule that declares `scope: changed`, separate from `out_of_scope`, with one `info` notice per such rule; `waiver.matches-nothing` leaves a waiver on such a rule eligible and unexamined; `urzua rules` prints each rule's `options` and `judges`. The `embodiment.locator-on-back-pointer-path` rule-table row is regenerated. **Why:** `MILE-203`, decided in `ADR-90`. | **substantive** |
> | 2026-11-06 | Bumped to `0.108`. Adds the opt-in rule `diff.decider-required` and `check --actors FILE` (`MILE-200`): a gated status move or protected-path change in a `--base` diff needs the pull request's author or a current approver in the gate's decider group, read from an actors file the caller supplies; `info` locally and an error under `require_evidence` with no file, `not-applicable` with no `--base`; it proves authorship only. A rule table row and two finding variants (exempt) are generated. **Why:** `ADR-89` section 4, accepted by the maintainer. | **substantive** |
> | 2026-11-06 | Bumped to `0.109`. For a type declaring no `known_fields`, the `edit-config` suggested by `config.pointer-field-not-known` and `config.relation-field-not-known` sets the full list the type's records use plus the field in question, instead of appending one field; two variants (`pointer-field-not-known-declare-set`, `relation-field-not-known-declare-set`) are added. **Why:** maintainer decision of 2026-11-05: applying the one-field append switched `header.field-set-consistency` on and flagged every other optional field. | **substantive** |
> | 2026-11-06 | Bumped to `0.110`. `relation.target-status-undeclared` and `config.companion-declaration-missing` follow the same rule as the two field-not-known rules: for a type declaring no `known_fields` the suggested `edit-config` is a `set` of the full list the type's records use plus the field(s) in question (`-declare-set` variants), not an `append` that would create a one-item list. **Why:** review of the same-day change found the same one-field append in these two rules. | **substantive** |
> | 2026-11-06 | Bumped to `0.111`. `embodiment.locator-promotion-candidate` lists the `scope` option (`ADR-90`, `MILE-203`): with `scope: changed` a locator is reported only when a record the `--base` diff adds or changes cites it, the finding sits on that changed record and names every citer, a plain `check` reports none, and the unchanged citers are counted in `deferred_by_scope`. The rule's table row is regenerated. **Why:** a repository that enables the nudge at `info` should not see it on records nobody edited; ADR-90 permits the option on a record-level rule. | **substantive** |
> | 2026-11-06 | Bumped to `0.112`. A tracked record missing from the working tree and not staged for deletion is judged as removed and named in a `missing-record` warning notice, where it made the whole run `not-run`; a `--rules` selection reads the back-pointer sources and published boundary files only for the rules that read them. **Why:** Phase 0 hardening from a second adopter switchover -- one `rm` stopped every check and the editor hook could not judge, and a header-only rule paid for scanning every source file. | **substantive** |
> | 2026-11-07 | Bumped to `0.113`. §Discovery states `discovery.include_untracked` (`SPEC-3`): with it, untracked, non-ignored record files directly in a declared directory join the run's files (source still `tracked-sweep`), `--base` counts them as added, and a named file keeps its `BUG-24` treatment; without it, a plain `check` adds the info notice `discovery.untracked-skipped` naming the untracked records it skipped. `scope` gains the always-present `include_untracked` and `untracked_included`. The info notice `discovery.untracked-not-drift-judged` counts records with a locator and no history that drift did not judge. **Why:** `MILE-206` / `ADR-92`; `ADR-56` stands for the default and carries the amendment. | **substantive** |
> | 2026-11-07 | Bumped to `0.114`. Adds the opt-in rule `identity.open-pr-collision` (`ADR-93`, `MILE-89`): under `--base`, a warning for an added record whose type and number a fetched pull-request ref under `refs/urzua/pr` holds with a different `Stable-Id`; `not-applicable` without `--base`; a head carrying the record's own `Stable-Id` is not a collision; `check` never fetches, and with no refs fetched the rule adds a notice saying how and its population (unit `pull-ref`) counts zero. The rule table is regenerated. **Why:** `MILE-89` / `ADR-93`. | **substantive** |
> | 2026-11-07 | Bumped to `0.115`. `urzua rules` prints a fifth `judges` value, `anchor-scoped`, for a rule that reads other records as context and reports only on changed ones; `embodiment.locator-promotion-candidate` carries it and, under `scope: changed`, emits one finding per changed citer, so `check <path>`, `--stdin-path` and a file-scoped waiver each see the finding on their own record. **Why:** the accepted 2026-11-07 amendment to `ADR-90`, points 4 and 5. | **substantive** |
> | 2026-11-07 | Bumped to `0.116`. Documents that `embodiment.locator-exists` reports `Realized-by` text outside a recognised entry as `locator-malformed`; the `undeclared-field-case` variant of `header.field-set-consistency` (rename, never remove); that `revision-log.change-class-required` reads an alignment row as a delimiter; and that `header.required-fields` leaves a type with no `required_fields` out of its population. **Why:** Phase 0 hardening review of the core rules. | **substantive** |
> | 2026-11-07 | Bumped to `0.117`. `identity.open-pr-collision` names the remedy that works: rename the file and its H1 to the next free number, or waive the finding when the other pull request is abandoned (`urzua migrate ids` cannot renumber); the `open-pr-collision` action reason says the same. An unknown `harness.guardrails` name is a load error, so `check` exits 2 on it. **Why:** `ADR-93` and `ADR-89` amendments of 2026-11-07. | **substantive** |
> | 2026-11-07 | Bumped to `0.118`. Documents the status computed from Done-means items for a type that declares `statuses.delivery.items`: the new `status.delivery-agreement` variants, among them the guaranteed `DoneWithUnresolvedItems` error (severity floor, unwaivable by a rule-level waiver, two declared actions); the new rules `status.done-items-shape` and `status.waived-item-stale`; the `item-locator-defect` variant of `embodiment.locator-exists`; the `severity_floor` and `waivable` keys of `urzua rules`; the roadmap's `items` object. **Why:** `MILE-214`, the engine of `ADR-96`; an undeclared config is unchanged. | **substantive** |
> | 2026-11-07 | Bumped to `0.119`. `revision-log.change-class-required` accepts the classes a type declares in `revision_log.classes` (default `substantive` and `structural`) and lists them in its finding, and reads the class and date from the columns `class_column` and `date_column` name; a header lacking a declared column or repeating it is the new variant `revision-log-column-unreadable`, offering `edit-section` `set`, with no row judged. `revision-log.version-bump-required` reports a record with such a log, now or at the merge base, as unreadable and no longer reads an unreadable base log as empty. The rule-table and action-table rows are regenerated. **Why:** `MILE-213`, the build of `ADR-94`; an undeclared config is unchanged. | **substantive** |
> | 2026-11-07 | Bumped to `0.120`. `urzua rules` also prints each rule's `scope`, `population_unit` and `spec`, so `urzua explain --rule` (`SPEC-13`) can state them; additive fields. **Why:** maintainer decision of 2026-11-07 (the `ADR-96` amendment of that date). | **substantive** |
> | 2026-11-07 | Bumped to `0.121`. Documents that a dependency caps the status computed from Done-means items: a record whose items all resolve beside an unfinished dependency agrees with `partial_status`, the finding and the `fix` notice name the capping dependency, `sequence.status-agreement` skips a blocked record whose `partial_status` comes from the cap, with an info notice (`status-blocked-capped`), and still reports one in progress with an unresolved item; `delivery-partial-blocked` applies only to a record with an unresolved item. **Why:** the `ADR-96` amendment of 2026-11-07, accepted by the maintainer, after `sequence.status-agreement` and `status.delivery-agreement` left no status that passed; a configuration without `delivery.items` or a sequencing pair is unchanged. | **substantive** |
> | 2026-11-07 | Bumped to `0.122`. `header.field-set-consistency` gains the optional `extension_prefixes` option: a record type mapped to its own extension prefix, used for that type in place of the fixed `x-`, matched in any case. A type the option does not name keeps the rule-level `allow_extension_prefix` behaviour exactly, and an unprefixed undeclared key is still a finding for every type. A prefix is non-empty, holds no whitespace and does not start a field the type declares, and each type named must be declared with `known_fields`; the rule-table row is regenerated from `RULE_METADATA`. **Why:** `MILE-141` Done means item 3, built on the maintainer's decision of 2026-11-07, which reverses the 2026-11-04 choice to leave the prefix fixed at `x-`. | **substantive** |
> | 2026-11-07 | Bumped to `0.123`. `type.dir-matches-nothing` also reports each status a type's `sections.exempt_statuses` lists that none of the type's records holds (variant `exempt-status-matches-nothing`, `edit-config` `set`). **Why:** `MILE-7` -- an exemption that sets nothing aside was a declaration that silently did nothing. | **substantive** |
> | 2026-11-07 | Bumped to `0.124`. Documents that a record's `Kind` selects the tiers that count as delivered for a type declaring `delivery.items.kind`, that `cited_records: success` makes a cited record count only at a `success` status of its own type (so a Proposed ADR or a Draft spec evidences nothing), and that `status.done-items-shape` reports a `Kind` outside the vocabulary; the rule table row is regenerated. **Why:** the `ADR-96` amendment of 2026-11-07, accepted by the maintainer (`MILE-73`). | **substantive** |
> | 2026-11-07 | Bumped to `0.125`. Adds the opt-in rules `embodiment.claim-expired` (a claim expires after a period declared per record type, status and evidence tier, with a first-cycle `spread_days` offset; its finding offers `urzua reverify --ids <record>`) and `embodiment.claim-renewal-unattested` (on `--base`, an anchor date that moved with no `Re-verified` row, or was removed), both `ADR-97`; a `--base` run reads the merge-base copy of the changed records of the types the expiry rule lists. The rule-table and action-table rows are regenerated. **Why:** `MILE-129`, the build of `ADR-97`; undeclared, nothing changes. | **substantive** |
> | 2026-11-07 | Bumped to `0.124`. Adds the opt-in rules `embodiment.claim-expired` (a claim expires after a period declared per record type, status and evidence tier, with a first-cycle `spread_days` offset; its finding offers `urzua reverify --ids <record>`) and `embodiment.claim-renewal-unattested` (on `--base`, an anchor date that moved with no `Re-verified` row, or was removed), both `ADR-97`; a `--base` run reads the merge-base copy of the changed records of the types the expiry rule lists. The rule-table and action-table rows are regenerated. **Why:** `MILE-129`, the build of `ADR-97`; undeclared, nothing changes. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/check.rs@sha256:c819006495f2, rust/crates/urzua-core/src/rules.rs@sha256:355dd5bf590d, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833, rust/crates/urzua-cli/tests/fail_on_integration.rs@sha256:4b1707f3b18a Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.126`. The pins on this spec's `Realized-by` locators were re-baselined against the files as they are after the sharded CI and the milestone `Kind` amendment landed on main (the `Re-baselined` row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97`: every stale pin is re-pinned with its real delta before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.127`. The `Realized-by` pin on `check_integration.rs` was re-baselined (the `Re-baselined` row above) after its tests were changed to name real tracked files; no behaviour this spec describes changes. | **structural** |
> | 2026-11-08 | Bumped to `0.128`. `embodiment.claim-expired` judges a record whose evidence is only its Done-means item evidence clauses, counting its age from the first evidence line. **Why:** `ADR-97` amendment of 2026-11-08 -- such a milestone never expired and `urzua reverify` could not renew it. | **substantive** |
> | 2026-11-08 | Bumped to `0.129`. `embodiment.locator-exists` does not examine an item whose valid, unexpired waiver holds while its evidence does not resolve (an expired waiver no longer excuses it) (`ADR-96`), and never probes a locator that resolves outside the repository; item messages name the type's declared `items.section`. **Why:** round-4 review of the delivery items. | **substantive** |
