# Changelog

All notable changes to `urzua` are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Written for the person installing it — what changed
for you — not a commit dump.

## 0.4.0 (2026-11-14)

0.4.0 turns urzua into a configuration-driven engine: every rule, status vocabulary, header field and
gate is declared in `.urzua/config.yaml`, and the engine no longer carries any record-type
vocabulary of its own. This is a curated summary. The complete itemised list is in
[docs/reference/changelog-0.4.0-full.md](https://github.com/urzuahq/urzua/blob/main/docs/reference/changelog-0.4.0-full.md).

### Breaking Changes

Read these before upgrading.

#### The configuration is `.urzua/config.yaml` at `schema_version: 2`, and every rule is opt-in

A `0.3.0` config does not load. An absent rule is off, not on, so every rule you want must be named
in the `rules` table. Severity is declared per rule with `level: warn` or `level: error`. A rule that
takes options (`pointer.target-status`, `claim.status-agreement`, `narrative-field.stale`) fails to
load without them, and unknown rule names and misplaced options are load-time errors.

Migration: name every rule you rely on under `rules`. `urzua init` writes a starter rules table for an
adopted corpus. A config at the previous `schema_version` is upgraded in memory so `check` keeps
working, with one warning, `config.schema-version-outdated`; `urzua migrate config` rewrites the file
in place, keeping its comments, and lists every key it added, changed, renamed, moved or dropped. A
config outside that window exits 2 with a `schema_version_error` object (`found`, `supported`,
`current`, `migration`) naming the command to run. A version-1 record type that omits `header_shape`
is refused with an error naming the type, because version 1 read it as the removed `blockquote` shape.

#### `dir` means that directory, not its subtree

A type's records sit directly in its `dir`. Nested directories belong to no type, and the new opt-in
`type.record-outside-declared-dir` reports a record-shaped file that falls between types rather than
dropping it. Migration: list every directory that holds records under `dir` or `extra_dirs`.

#### The `blockquote` and `bold-list` header shapes are removed

YAML frontmatter is the one header shape with a reader (alongside `none` for a type with no header),
and `header_shape` defaults to `yaml-frontmatter`. A config declaring `header_shape: blockquote` or
`bold-list` fails to load with an error naming the removed shape, and a record still written in
either form is reported by `header.required-fields` as unreadable, with the form named. The
`header.layout-consistency` and `header.deprecated-shape` rules and the `header_layout` option are
removed; naming either rule is an unknown-rule error. Migration: rewrite un-migrated headers as YAML
frontmatter before upgrading. `urzua init` counts such records in a notice and `urzua migrate ids`
names the form when it skips one.

#### Header field names are compared exactly

Field names were compared case-insensitively, so a record writing `status` matched a config declaring
`Status`. Now a corpus relying on that gets a finding that names the declared spelling. Migration:
spell each header field as the type declares it. `header.field-case-mismatch` is the one place that
reports a declared field written under a different case. Comparison is exact because every correct
case-insensitive comparison makes `Maße` and `Masse` one field, and accepting an undeclared name is a
missing finding. Fixes BUG-97.

#### `claim.status-agreement`, `pointer.target-status` and `narrative-field.stale` read `Status` only where it is declared

They read a target record's `Status` only when that record's type declares `Status` in
`required_fields` or `known_fields`. Migration: a type that relied on the old unconditional check must
declare `Status` to keep these three rules' coverage. Fixes BUG-109.

#### The report contract changed

- `rules_executed` entries no longer carry `records_examined` or `scope`. Every rule reports a
  `population` instead: what it was eligible to examine and what it judged, in a named unit
  (`record`, `field`, `record-type`, `path`, `claim`). Migration: read `population.examined` and check
  `population.unit` before comparing it to a record count. The report also gains a top-level
  `records_read_by_any_rule`.
- Each `population` gains `out_of_scope`, the candidates the configuration cannot reach, as distinct
  from ones the corpus has not written yet. It is always present, including as `0`.
- `FixReport.records_examined` and `MigrateSchemaReport.records_examined` are replaced by
  `population`. `migrate schema --report` now excludes a record with an unparsed header from
  `would_fail` and discloses it as a `Notice`.
- A run that establishes nothing is reported as `not-run`, not `ok`, and a configuration with no rules
  or only configuration-reading rules no longer exits 0 as though the corpus were checked. `check`
  and `audit` no longer report `not-run` just because no record-scoped rule ran, so a thin
  configuration earns a clean exit and says `records_read_by_any_rule: 0`.
- `check` and `audit` emit the declared `scope.source` value, not a Rust `Debug` rendering.

#### Unreadable records and path arguments

A record the tool cannot read now stops the run instead of dropping out of the corpus with a clean
verdict (a deletion staged in git is still a legitimate absence). A path argument narrows what is
reported on, not what a reference resolves against, so `check docs/adr/` no longer reports every
reference pointing outside the path as dangling.

#### `urzua audit` is deprecated

`audit` is an alias of `urzua check` over your configured rules, so a CI step running it now fails on
a blocking finding from any declared rule. It adds one deprecation notice and is removed in the next
release. Migration: use `urzua check`, or `urzua check --rules pointer.resolution,relation.supersession-reciprocity`
for the narrower report.

### New commands and features

#### Waivers from a report: `urzua waive --from-report`

Runs `check`, then previews one draft waiver per rule and file for the unwaived findings at or above
`fail_on`. Each draft has a pending `Reason` and no `Expires`, so it suppresses nothing until a person
fills it in. The output lists drafts and skipped findings with the reason. `--apply` writes the drafts,
numbered under the same lock `urzua new` takes, so a second `--apply` writes nothing. It refuses
(exit 2) when a draft could take effect without a required `Reason` and an `Expires` or `permanent`.
`waiver.matches-nothing` reports a waiver that is expired, names an unknown rule, or hides nothing.
A suppression type can declare `permanent` and `effective_requires`.

#### The MCP server: `urzua mcp`

A read-only Model Context Protocol server on stdio, so an agent can read the corpus without a shell.
Tools: `urzua_check`, `urzua_list_records` (filter by record type and by status),
`urzua_show_record` (one record with the relation edges that touch it),
`urzua_get_findings_for_path` (which records govern a path) and `urzua_get_graph` (every relation
edge). Each returns the report the matching command prints, and none writes. See `docs/reference/mcp.md`.

#### `next_action` in every report

Every command's JSON carries a top-level `next_action` (one runnable command such as
`urzua fix --apply --ids docs/adr/ADR-1-a.md`, or `null`) and `next_action_reason` (such as
`repairs 3 of 5 findings`, `null` exactly when `next_action` is). Findings can also carry a
`suggested_action` as data.

#### Scoped checks

- `urzua check <path>` reads only the source files the back-pointer rules need. A single-record run
  went from 4.61 s to 1.91 s on a 2,361-file repository. Files not judged are counted as
  `deferred_by_scope` with an `info` notice.
- `urzua check --base <ref>` reports on the records changed since the merge base, and on records that
  name or cite a changed or deleted record. `scope` says what was examined. Built for pull-request
  gates over a large corpus.
- `urzua check --rules <id>[,<id>...]` runs only the named rules; `check --stdin-path` judges proposed
  content of one record; a rule can declare `scope: changed`.
- A scoped check no longer reports `not-run` beside `blocking: true`.

#### Greenfield `urzua init`

`urzua init --types adr,rfc,spec,prd` sets up a repository with no records, from built-in profiles
written into `.urzua/config.yaml` as editable rules, with starter templates in `.urzua/templates/`.
`--dir` sets the layout root (default `docs`). On a terminal it asks which types to govern. Re-running
with the same selection changes nothing. For an adopted corpus it proposes one type per directory
holding records, with the filename prefix actually used.

#### `urzua migrate filenames`

Renames record files whose names do not match the type's filename convention, keeping directory,
number and slug so ids and `Stable-Id` do not change. It rewrites relative links and path mentions,
lists `unrewritten` and `skipped` files, and writes nothing without `--apply`. A second applied run
changes nothing. `filename.identity-pattern` reports off-pattern files.

#### Drift baseline

`embodiment.consistency` takes `drift_baseline: {ref: v1.0.0}`: drift that already existed at the ref
draws no finding and does not block, and is listed in an `embodiment-drift-baselined` notice. Also
`drift_grace_days` (with an `embodiment-drift-in-grace` notice), `drift_audit_field`, per-category
drift policy under `embodiment.claim-expired`, and a global `--as-of YYYY-MM-DD`.

#### Section rules

Per-type `sections` declarations with opt-in rules: `section.required`, `section.shape`
(`y-statement`, `table-min-rows`), `section.duplicate`, `section.level-skipped` and `section.order`
(from the type's template). Required sections can declare `aliases` and `preamble: true`, and
`exempt_statuses` sets records aside. `urzua init` does not propose these rules.

#### `code.dangling-bug-reference`

An opt-in rule that reports a source comment citing a record id that resolves to no record, the mirror
of `embodiment.locator-exists`. It reads files under `back_pointer_paths` with the declared
`extensions` and takes `external_ids` and `ignore_marker`.

#### `urzua doctor`: `template-sections` and hook wiring

`doctor` reports each required section a type's template has no heading for, and whether the
`pre-commit` and `pre-push` git hooks are wired (a `hook-wired` check with a `state`).

#### The decider gate and authority

`authority` declares decider groups of `logins`, the `evidence` that may name one (`pr-author` or
`pr-approver`), and `gates` by record type and status or by path. `diff.decider-required` with
`urzua check --actors FILE` reports a gated move or changed gated path whose author or approvers are
not in the gate's group, judged against the merge base. Deleting a gated record counts as a gated
move, and statuses compare case-insensitively. `authority.decider_table` (`base`, `head`, `both`) says
whose table applies, and `authority.waivable` (`never`, `base`, `any`) says whether a waiver can
silence the rule. The rule proves authorship, not who decided. The reusable `urzua-gate` GitHub
Action installs a pinned release, verifies its build provenance, runs `check` and `check --base`, and
posts annotations and one updated pull request comment.

#### The agent harness

`urzua harness install --adapter NAME` writes an adapter's hook entries, a translator and a lock file
without touching other settings, and keeps one marked block of instructions. `harness run edit`
refuses an agent's write to a gated record or path (a refusal no override lifts). `harness run
post-command` checks changed records after any shell command and never blocks. Hook commands embed
the translator's sha256 and refuse to run on a mismatch. `harness.outside_writes` and
`harness.allow_paths` cover writes outside the repository; `block_on` and an `override_log` with the
`override.unreviewed` rule cover overrides. Install also writes a drafting command per record type
and the lifecycle commands `/amend`, `/repin` and `/done`.

#### Stale-pin and evidence checks

- `Realized-by` evidence can carry `@sha256:` pins and scopes: `code:path#Symbol`,
  `code:path#Type::member`, `code:path#L10-L40`, resolved by `locator_resolvers` you declare. A scoped
  pin drifts only when its item changes. `urzua explain <path>#<scope>` prints the span and the
  locator to paste.
- `urzua fix --rebaseline` lists records whose pinned locator changed; with `--apply`, `--ids`, `--by`
  and a required `--reason` it re-pins and writes an attributed revision row. `--actor agent|human`
  and a top-level `attestation` key set the recorded identity.
- `urzua reverify` renews an expired claim (`embodiment.claim-expired`, with per-type expiry), and
  `embodiment.claim-renewal-unattested` checks that renewals are real.
- `embodiment.locator-exists`, `embodiment.back-pointer-resolution` (`Implements:` and `Verifies:`
  comments), `embodiment.back-pointer-decision-accepted` and `embodiment.requires-accepted-status`
  judge the evidence itself. `statuses.delivery` holds a `Status` to delivery evidence.

#### Other additions

- Record-type declarations: `statuses` vocabulary (with `require`, `forbid`, `pending`, `initial`,
  `progress`, `implementable`, `claim_retained`), `identity`, `extra_dirs`, `revision_log`,
  `relation_fields`, `relation_pairs` (with `acyclic`, `types`, `provisional`), `companion_fields`.
- New opt-in rules including `identity.collision`, `field.pending`, `field.untrimmed-value`,
  `date.field-valid`, `date.field-order`, `header.case-variant-key`, `index.freshness`,
  `citation.resolution`, `citation.issue-exists` (with `urzua refs refresh`), `citation.path-exists`,
  `boundary.published-path-reference`, `sequence.cycle` and `sequence.unresolved-blocker`.
- `urzua roadmap` computes waves, the ready set and the critical path from the records alone.
- `urzua export --format json`, `urzua index` to regenerate declared indexes, `urzua explain --rule`,
  `fail_on`, `level: info`, and `urzua new` improvements (no number reuse across branches or open pull
  requests, declared identity and fields, `Stable-Id` always written).

### Notable fixes

- Discovery reads git's output NUL-separated, so a record with a non-ASCII filename is part of the
  corpus (BUG-87).
- A rule that was handed nothing no longer certifies the corpus `ok` (BUG-94).
- `urzua new` no longer reissues a number already in use, including for hyphenated and
  digit-bearing type prefixes.
- `urzua init` no longer writes a config that cannot load, and no longer proposes rules the corpus
  cannot satisfy.
- `urzua fix --apply` no longer corrupts a record whose `Embodiment` is `null`, `~` or empty, and
  `fix --apply` and `migrate ids --apply` change only the lines they mean to (CRLF is kept).
- The agent harness no longer lets through an edit it could not judge, or one whose hook runs from
  another directory.
- Every date compared is a UTC date, so a waiver no longer lapses a day early or late by time zone.
- A panic exits 2 (the documented "could not run" code), not 101.
- The `urzua-gate` action verifies the downloaded binary's build provenance and version.
- A YAML frontmatter header is read as written: `1.10` and `0x1F` are no longer reinterpreted.
- Paths read or written on a name from arguments or configuration are confined to the repository,
  symlinks included.
- Commands outside a git repository fail with a clear message, and `check`, `audit`, `fix` and
  `reverify` run `git blame` once per record file.

See the [full itemised changelog](https://github.com/urzuahq/urzua/blob/main/docs/reference/changelog-0.4.0-full.md)
for every change, including the bug numbers fixed.


## 0.3.0 (2026-09-16)

### Breaking Changes

#### `check` and `audit` emit the declared `scope.source` value, not a Rust `Debug` rendering

`scope.source` came back as `"GitTracked"` — the `Debug` rendering of an internal enum, a value that
appears in no specification. SPEC-2 declares four values for this field, and **no run ever produced
one of them.** An agent branching on `scope.source == "tracked-sweep"`, exactly as the spec
documents, took the wrong branch on every invocation.

It now emits `"tracked-sweep"`.

The field is a serde-renamed enum rather than a `String`, so renaming an internal type can no longer
change published output silently. Only values the tool can actually emit exist on that enum — SPEC-2's
table now marks which of its four declared values a run produces today, so a consumer is not misled
into branching on a mode that isn't built.

**Breaking** for any consumer matching the old `"GitTracked"` string. Nothing could have matched it
correctly against the documented contract, since it was never a documented value.

## 0.2.1 (2026-09-16)

### Fixes

#### `filename.title-consistency` reports the right cause, at the right line, and stops inventing findings

Three defects in one rule, all of which only show against a corpus this project did not author:

- **An H1 that exists but carries no number** (`# Add Status Field`) reported `no H1 title found`.
  Two different defects — "add a title" and "this corpus numbers its records somewhere other than the
  H1" — shared one message, and it was false for the second. Each cause now has its own message, with
  the no-H1 wording unchanged so that case is not a behaviour change.
- **Every finding pointed at line 1.** Records here open with YAML frontmatter, so an H1 is never on
  line 1. Findings now carry the line the heading was actually found on; the no-H1 case carries no
  line rather than a fabricated one.
- **A `# ` inside a fenced code block counted as the title.** A record whose only `# ` line was a
  shell comment reported a confident mismatch against a number that was not a record number. Fenced
  regions are now skipped, as is YAML frontmatter — whose comment lines start with `# ` for the same
  reason.

Separately, **a waiver whose `Expires` value is not a date no longer suppresses anything.** The
comparison was lexical and unvalidated, so `Expires: soon`, `TBD`, or a mistyped `2026-9-16` sorted
above every real date and produced a waiver that could never expire. A non-date expiry now expires
immediately. This mechanism may fail toward more findings, never fewer.

## 0.2.0 (2026-09-16)

### Breaking Changes

#### `pointer_fields`/`narrative_fields` are now config-declared per type

`pointer_resolution`, `header.pointer-field-clean`, and the renamed `narrative-field.stale` (was
`blocked-on.stale`) previously scanned a fixed Rust array — `Implements`/`Derives-from`/`Parent`/
`Blocked-on` — regardless of what a type's `.urzua/config.toml` entry actually declared. An org
adding a custom relationship field (e.g. `Feeds-into`) would pass `header.field-set-consistency`
but never get it resolved, checked for dangling references, or shown in `urzua graph`.

Both are now real per-type config keys, `pointer_fields` (clean, comma-separated references,
format-enforced) and `narrative_fields` (prose-tolerant, staleness-checked) — undeclared means zero
fields of that kind checked, no implicit default. **If your own `.urzua/config.toml` doesn't declare
either key for a type, that type silently loses the pointer-resolution/clean-format checking it used
to get for free from the old hardcoded field list** — add `pointer_fields`/`narrative_fields`
explicitly to keep the same coverage; three new `check` rules will otherwise stay silent on an
undeclared type rather than warning you. `config.pointer-declaration-missing` (a type must declare
both lists explicitly, even as `[]`), `config.pointer-field-not-known` (a declared field must also
be in `required_fields`/`known_fields`), and `config.pointer-narrative-overlap` (a field can't be in
both lists).

`urzua graph` gains a `kind: "pointer" | "narrative"` field on every edge, is now config-driven
instead of a hardcoded 3-field list (so a type's own `Parent` now appears for free), and no longer
reports a false `dangling: true` for a padded reference against an unpadded filename (a normalization
gap `pointer_resolution` never had).

#### Legacy pre-type-prefix filenames no longer resolve

`record_id`/`filename_number`/`next_display_number` now recognize only the `TYPE-NNNN-slug.md`
filename shape. The legacy `NNNN-slug.md` shape (accepted permanently by ADR-36's original
decision) is no longer parsed -- a reference to a filename in that shape is now correctly dangling,
the same as a reference to any other nonexistent record. BUG-9 found this repo's own corpus, and
every other known corpus, has zero files still using the legacy shape.

#### One shared output contract across every command: `Report`, `Notice`, `emit()`

`check`, `audit`, `fix`, `new`, `explain`, `graph`, and `doctor` previously each hand-rolled a
different, incompatible JSON shape. Every one of them now implements a shared `Report` contract and
prints through one function -- concretely:

- Every report gains an optional `notices` array: non-fatal observations (`severity: "info"|
  "warning"`, `subject`, `message`) that never affect the exit code, omitted entirely when empty.
  `--by`'s divergence from an authenticated `gh` login is the first real user of this.
- Every fatal "could not run" path now emits real JSON on stdout, not just an unstructured stderr
  message -- previously true only for `fix`; `new`, `explain`, `graph`, and `doctor`'s early guard
  printed nothing on stdout at all on failure.
- **`check`/`audit`'s failure-path JSON changes shape**: it used to include `scope`/`rules_executed`/
  `files_examined` as present-but-empty/zero keys even when the tool couldn't run at all; those keys
  are gone on that path now (`{"status": "not-run", "error": "..."}` instead). Every other command's
  change is additive (nothing on stdout before, real JSON now).
- Stderr is no longer used for anything this tool's own code controls, including genuine errors --
  the error message now lives in the JSON itself. `--help`/`--version` remain plain text on stdout
  (never JSON-wrapped), matching `cargo`'s own convention. An unexpected panic now prints a minimal
  JSON object to stdout instead of Rust's default raw text to stderr.
- A bad CLI flag now produces the same JSON-on-stdout, real-exit-code behavior as any other fatal
  error, instead of clap's own unstructured usage message.
- `urzua init` and `urzua migrate ids` join the contract too: `init` reports `proposed`/`written`,
  with the full rendered config only present on `--dry-run` (nothing else to read the preview from,
  since nothing was written); `migrate ids` reports `missing` and, when `--apply` runs, a real
  per-file `applied`/`skipped`/`failed` outcome instead of `[OK]`/`[SKIPPED]`/`[FAILED]` prose lines.
  `migrate ids --apply` also now exits 1 if any file failed to write, matching `fix --apply`'s own
  signal for the same shape of outcome -- previously exit 0 unconditionally, even on a real write
  failure.

#### Stdout is always JSON -- `--format` is gone

`check`, `fix`, `explain`, `graph`, and `migrate schema --report` no longer support `--format`.
Stdout is unconditionally one JSON object, on every invocation, with no human-readable rendering
anywhere (stdout or stderr). If you were parsing `--format human` output or relying on a stderr
rendering, switch to parsing the JSON on stdout -- it was already the more complete report.

### Features

#### `urzua audit`: supersession reciprocity and dangling references

`urzua audit` reports cross-record issues `check` doesn't scope to: a record claiming to supersede
another that doesn't point back, and cross-references that don't resolve. Read-only -- never writes,
since a bulk cross-reference rewrite is a real data-loss risk without a review step.

#### `check` now detects Embodiment drift

`embodiment.consistency` checks whether a `Realized-by` locator changed, per git history, since
the `Realized-by` line was last touched -- if so, the expected `Embodiment` is `Drift detected`
regardless of tier, and a stated value that disagrees is a finding. No new schema field: nothing is
stored, the comparison reads git blame/log directly. Requires full git history to detect anything
-- a shallow clone (CI's default checkout depth) makes this rule silently unable to find drift.

#### `urzua explain` and `urzua graph`

`urzua explain <path>` lists every record whose `Realized-by` names that file as evidence -- which
decisions govern this file. `urzua graph` dumps the full `Implements`/`Derives-from`/`Supersedes`
relationship graph as data, flagging edges that don't resolve.

#### `urzua new` now emits type-prefixed, unpadded filenames

New records are named `TYPE-N-slug.md` (e.g. `ADR-36-...md`) instead of `NNNN-slug.md` --
matching how a record is already referenced everywhere else (`Implements: ADR-36` is now the same
string as the file's own name). The number is never zero-padded: fixed-width padding doesn't solve
lexicographic sort order permanently, it only defers the break to whenever one type crosses the
padded width, and breaks worse there (mixed-width filenames). Unpadded is at least consistent
forever. The descriptive slug is kept.

Existing `NNNN-slug.md` filenames are never renamed and keep working forever -- both shapes resolve
identically regardless of padding, and a directory can hold a mix of both indefinitely. Also fixed:
`filename.title-consistency` had its own separate hardcoded 4-digit-only parsing (the same defect
class as BUG-2, in a different function) -- both now accept any digit length and compare by numeric
value.

#### `urzua fix`: detect and apply Embodiment repairs

`urzua fix` reports fields whose stated value disagrees with what Urzua computes from
`Realized-by` evidence. `urzua fix --apply --ids <records> --by <you>` writes the computed value
back -- one field's line only, with a required identity and an appended revision-log entry. Refuses
outright on a record with no revision log, rather than writing somewhere it can't be audited.

#### Two more header shapes: bold-list and YAML frontmatter

Declare `header_shape = "bold-list"` or `"yaml-frontmatter"` per record type in
`.urzua/config.toml` alongside the existing `blockquote` default. Lets `check`/`fix` read a corpus
that already uses a bold markdown list or real YAML frontmatter, instead of forcing a rewrite to
adopt Urzua.

#### `fix --apply`/`new` now prefer an authenticated `gh` session over `--by`; `new` gains a `--by` flag

Identity resolution for `fix --apply` and `new` (RFC-2/ADR-31) now checks `gh api user` first, then
falls back to an explicit `--by` value, and only then to `git config user.name` if neither is
available. Previously `--by` won outright, which meant a typed name could silently override an
authenticated `gh` session -- that no longer happens. If a `--by` is given and it differs from an
authenticated `gh` login, `gh`'s login still wins; the divergence is surfaced as a `notices` entry
in the command's JSON output rather than the value being silently discarded. `git config user.name`
is equally unsigned local input as `--by`, so it no longer outranks it (ADR-31's amendment).
`urzua new` also gains the `--by` flag `fix --apply` already had. `gh api user` is also now bounded
to 5 seconds rather than hanging indefinitely if `gh` doesn't respond.

None of these sources is a cryptographic attestation -- each is just harder to spoof by accident
than the one below it (RFC-2's own stated goal is raising the cost of a careless rubber-stamp, not
achieving unforgeable proof). Treat the resulting `Author`/`Tool-authored (by ...)` value as
attribution, not verification of who actually reviewed the change.

#### `urzua migrate ids`: backfill stable IDs

Backfills a collision-free `Stable-Id` (ULID) into every record that lacks one, without touching
the filename or any cross-reference -- the display number stays exactly what it already is.
Dry-run by default; `--apply` writes.

#### `urzua migrate schema --report`: preview a new required field

`urzua migrate schema --report --field <Name>` lists every record that would newly fail if `Name`
were added to `required_fields` today -- a read-only preview before you ever touch config.

#### Record types can now declare a shorter filename prefix

`RecordTypeConfig` gains an optional `prefix` field, decoupling a type's filename/ID prefix from
its own name -- the same way `dir` already decouples the type name from its directory. Omitted, a
type's prefix still defaults to its name upper-cased (no change for existing configs).

This repo's own `milestone` type now declares `prefix = "MILE"`: `urzua new milestone "..."` still
takes the same argument and `docs/milestones/` is still the directory, but new records are named
`MILE-N-slug.md` instead of `MILESTONE-N-slug.md`. All 36 existing milestone records were renamed
to match, with cross-references updated.

#### `urzua new`: create a record with a stable ID assigned

`urzua new <type> [title]` fills in the type's checked-in template (or synthesizes YAML
frontmatter if the type declares that shape and has no template) with a fresh `Stable-Id`,
resolved author, and today's date. Never asks you to pick a number.

#### `check` now flags pointer fields that mix in explanatory prose

New rule, `header.pointer-field-clean`: `Implements`/`Derives-from`/`Parent` are meant to hold only
comma-separated reference IDs, but nothing previously caught a value like `RFC-1 (Accepted)` or
`SPEC-1 (v0 CLI), which lists the bug classes this suite must reproduce.` -- `extract_references`
silently reads the leading token and discards the rest. `Blocked-on` is deliberately excluded: it
legitimately mixes free text with an optional embedded reference.

#### `blockquote`/`bold-list` deprecated; `init` now proposes `yaml-frontmatter`

`urzua init`'s adopt mode now proposes `header_shape = "yaml-frontmatter"` for every newly-adopted
record type, regardless of what shape the existing corpus already uses -- it recommends the
destination going forward rather than preserving whatever the corpus happens to look like today. A
new non-blocking rule, `header.deprecated-shape`, warns when a configured type's `header_shape`
isn't `yaml-frontmatter`. Parsing support for `blockquote`/`bold-list` is unchanged and stays
indefinitely: an existing corpus, or a first-time evaluator's unmodified docs, still read correctly.

Also fixed: `render_synthetic_yaml` (what `urzua new` uses for any `yaml-frontmatter`-shaped type)
previously built its output with unescaped string formatting -- a real value containing a colon or
a numeric-looking string could round-trip incorrectly. It now serializes through a real YAML
mapping, and no longer adds `Date`/`Author` to a record unless the type's own `required_fields`
actually names them.

### Fixes

#### Fix: `check <path>` now actually scopes to `<path>`

`urzua check docs/adr/` previously examined the entire corpus regardless of the path given --
`paths` was used only to locate the repository root, never to filter what got checked. Fixed:
`check` now genuinely restricts examination to files under the requested path(s). No CLI syntax
change; results narrow correctly for the first time.

Also fixed: record identifiers no longer require exactly 4 digits in the filename, and reference
matching (`Implements`/`Derives-from`/`Supersedes`) now compares by numeric value, so `ADR-34`
and a hand-typed `ADR-34` resolve to the same record regardless of padding.

#### `urzua new` respects the configured header_shape, even with an old template present

`urzua new` picked its output shape by whether `.urzua/templates/<type>.md` existed, ignoring
`header_shape` entirely -- a type configured as `yaml-frontmatter` with a leftover blockquote
template still got blockquote output (BUG-3). The configured shape now wins unconditionally: a
`yaml-frontmatter` type always gets synthesized YAML frontmatter, and if a template happens to
exist, its body sections (everything from the first `## ` heading onward) are still spliced in
underneath, so fixing the header shape doesn't cost the template's scaffolding.

#### `check` now surfaces the real reason a YAML header failed to parse

A record whose `yaml-frontmatter` header contained genuinely broken YAML (a value starting with a
reserved character, an ambiguous unquoted colon) previously reported only `"no header-shaped region
found"` -- the actual parser error, including its line number, was discarded. `header.required-fields`
now appends the real `yaml_serde` error (or a specific "must be a mapping" message when the YAML
parses but isn't the right shape) to that finding, instead of leaving a reader to diagnose the raw
bytes by hand.

## 0.1.0 (2026-09-05)

First tagged release. `urzua check` and `urzua init` (adopt mode) are real; `new`, `audit`,
`migrate`, `export`, and `import` still exit 2 with "not implemented yet."

### Added

- `urzua check`: validates a corpus against a declared shape — header-format consistency, cross-
  reference resolution (`Implements`/`Derives-from`), field presence/quality (blank vs. placeholder
  vs. pending vs. present), filename/title consistency, and `Supersedes`/`Superseded-by`
  reciprocity. JSON (`--format json`) and human-readable output.
- `urzua init`: adopt mode only — proposes a `.urzua/config.toml` from an existing corpus without
  moving any files. Never clobbers an existing config; `--dry-run` supported.
- `urzua doctor`: reports on the tool's own configuration health (does the config exist, does it
  parse, does CI actually invoke `check`) rather than on record content.
- Waivers: a reviewed exception to a rule is its own record type (`Rule`, `Scope`, `Reason`,
  optional `Expires`), never a config-level ignore list. A waived finding stays listed in output,
  just excluded from the blocking exit code.
- `.urzua/config.toml` carries `schema_version` from this release onward.

### Known limitations

- `new`, `audit`, `migrate`, `export`, `import` are unimplemented stubs.
- The full SPEC-2 rule set (roles, required sections, declared scope, boundaries) is not yet
  built — see `docs/specs/SPEC-2-urzua-check.md`.
