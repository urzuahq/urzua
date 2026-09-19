# Changelog

All notable changes to `urzua` are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Written for the person installing it — what changed
for you — not a commit dump.

## 0.4.0 (2026-09-19)

### Breaking Changes

#### Config is now YAML: `.urzua/config.yaml` replaces `.urzua/config.toml` (ADR-52).

Records were already YAML and the config was TOML, so an adopter met two formats
on day one. They are now one. `urzua-core` carried both parsers; dropping TOML
removes a dependency rather than adding one, and `toml` leaves the purity
allowlist.

**Breaking.** An existing `.urzua/config.toml` is not read, and nothing reports
that it is there: `ADR-54` decided against carrying migration diagnostics for a
format the tool no longer parses. Convert it to `.urzua/config.yaml` before
upgrading; a leftover TOML file is ignored, not removed.

`urzua init` writes `config.yaml`, rendered through the real serializer instead
of string concatenation, so a directory name carrying the format's
metacharacters can no longer escape its value.

#### Every rule is opt-in and carries a declared severity (`MILE-80`, implementing `ADR-53`).

`.urzua/config.yaml` gains a `rules` table. A rule that is not named there does
not run, and appears in `rules_executed` as `not-enabled` rather than being
omitted -- "off" and "ran clean" stay distinguishable. The declared `level`
(`off`/`warn`/`error`) replaces whatever severity the rule body chose: severity
was 34 hardcoded literals with no config key, despite `SPEC-1` listing it as a
v0 configuration surface.

`schema_version` is now 2. An older config fails with "schema_version 2 not
supported" rather than an unknown-field error.

**Breaking.** A config with no `rules` table runs no rules. `urzua init` writes
every rule at `warn`, so an adopted corpus is told what is irregular without
being blocked on day one.

`pointer.resolution` is split. It emitted a finding for every reference that
*resolved* -- 172 of 213 findings on this project's own corpus were the tool
announcing a reference worked. Resolution success now reports nothing; the
target's status is `pointer.target-status`, which fires only on statuses a
repository declares via `not_in`. One rule id cannot carry two severities, so
these could never be levelled apart while they shared one.

A rule name this build does not ship, or an option on a rule that does not take
it, is a load-time error naming the valid alternatives.

`field.quality` is split the same way (`BUG-38`). `Blank` and `Placeholder`
mean a required field was forgotten; `Pending` means someone declared the work
unfinished. Sharing one rule id left no correct setting -- `error` blocked CI on
a deliberate marker, `warn` stopped a genuinely empty field from blocking.
`field.pending` is now its own opt-in rule.

#### A record type's `dir` means **that directory**, not that subtree (`RFC-35`).

Prefix matching let two types claim the same record whenever their directories
nested, and nothing in the schema resolved the overlap. Four heuristics accreted
in `urzua init` guessing what an adopter meant by it, and each re-entered the
previous one's failure -- the last silently dropped every record held directly
in a directory that also had two record-bearing subdirectories, while `check`
reported success over them.

`urzua init` now proposes **one type per directory holding records**: no
folding, no containers, no inference. `docs/adr/archive` is proposed as its own
type, which an adopter keeps, deletes or merges. Every record is accounted for
exactly once.

**Breaking** for a config whose `dir` relied on matching a subtree. Recursion is
not implemented: no corpus examined needs it, and it is added when one asks.

An empty option value is also now rejected rather than accepted:
`closed_statuses: []` made every claim a violation, and `claim_paths: []`
scanned nothing.

### Features

#### `urzua init` can adopt a corpus that does not live under `docs/` (`BUG-36`), and

both halves of adoption now recognise the same filenames (`BUG-37`).

`detect_record_types` groups by each record's own parent directory from the
tracked set. A corpus at `doc/adr/` adopts as `dir: doc/adr`; previously `init`
refused, and proposing `docs/<dir>` regardless would have been worse -- `check`
would then examine zero files and report success.

`parse_record_filename` is one recogniser for both conventions, `0001-slug.md`
and `ADR-1-slug.md`, used by the adopt scan and by `urzua new`'s numbering.
They carried one each and accepted disjoint sets, so adopt mode could not read
the records this tool itself writes, and in an adopted Nygard corpus `new`
returned 1 beside an existing `0001-`.

Two hazards are handled explicitly: a proposed directory that contains another
is dropped, since discovery matches by path prefix and the outer one would
claim the inner one's records; and two directories ending in the same component
are qualified rather than emitted as a duplicate type name.

`urzua new` still writes `ADR-4-slug.md` into a corpus whose own convention is
`0004-slug.md`. Recognising both shapes is not the same as writing the one a
corpus uses; that is `identity.pattern`, in `RFC-33`'s document model.

#### New rule `claim.status-agreement`: a file outside the corpus that claims to

close a record, while the record itself says otherwise.

Written after a changeset in this repository announced it closed `BUG-36`. It
had not -- it fixed a hazard recorded *beside* that bug -- and nothing noticed,
because the claim and the record it contradicted live in different files and
only one of them was ever read.

Opt-in like every rule, with both options declared rather than inferred:

```yaml
rules:
  claim.status-agreement:
    level: error
    claim_paths: [".changeset"]
    closed_statuses: ["Fixed", "Done", "Accepted"]
```

A built-in status list would make the rule stop applying the moment a
repository used a status it did not know.

It matches on verb and proximity, not meaning, so it can be wrong in both
directions: a present-tense sentence *discussing* a claim will match, and a
claim phrased without one of the verbs will not. Only the present tense is
matched -- "closes X" is how a claim is written, while "announced it closed X"
is prose about one, and including the past tense made the rule fire on its own
changeset. A waiver (`ADR-11`) is the intended escape, deliberately a record so
an exception is visible rather than a silent pattern tweak.

#### New rule `embodiment.locator-exists`: a `Realized-by` locator naming a path that

is not in the working tree (`BUG-49`).

Nothing checked this. `embodiment.consistency` asks whether a locator's content
changed since the claim was written, so a locator naming nothing has no history
to compare and drifts past the one rule built to notice. The whole `Embodiment`
model rests on these paths -- `Verified` and `Implemented` are computed from
them -- so a record could claim verified work while naming a deleted file, and
the claim read as stronger than `Not started` rather than weaker.

Found live: one of this project's own records had named a file deleted two days
earlier, in a corpus the tool gates on every push.

Opt-in and levelled like every rule. The CLI supplies path existence, so
`urzua-core` stays pure.

#### `urzua audit` now honours the `rules` table, `urzua init` no longer drops a

record type, and an incomplete rule declaration fails at load.

`urzua audit` bypassed the rules table entirely (`BUG-42`). `MILE-80` routed
every rule in `check` through the opt-in gate and left `audit` calling two rules
directly, so a repository declaring `pointer.resolution: off` was still blocked
by `audit`, and a declared level was ignored. The gate is now shared rather than
owned by one command.

`urzua init` dropped the outer record type when a subdirectory also held
records (`BUG-43`). A corpus with three records in `doc/adr/` and one in
`doc/adr/archive/` adopted only the archive, leaving three records ungoverned
with `check` reporting success over them. The outer directory now wins and
nested records are folded into it.

A rule declared without an option it requires is now a load-time error
(`BUG-44`). `claim.status-agreement` without `closed_statuses` treated every
claim as a violation, including correct ones; without `claim_paths` it scanned
nothing and reported success forever. `init` no longer proposes rules it cannot
declare completely.

### Fixes

#### A reference with a possessive attached is no longer invisible (`BUG-39`).

`MILE-13` in this repository declares `Blocked-on: RFC-9's own Q2`. Both
extractors returned nothing -- `extract_references` rejected `9's` as
non-digits, and `scan_references` trims only non-alphanumerics from each end,
so the `s` kept the apostrophe attached. A real dependency was invisible, and
`narrative-field.stale` reported success on a field it could not read.

Narrowed deliberately: only a possessive is stripped. `RFC-9a` and `RFC-9s` are
different identifiers and stay unrecognised.

#### Four defects found by reviewing the release diff as a whole, two of them

regressions introduced by this release's own fixes.

A date-named file no longer hijacks `urzua new`'s numbering (`BUG-53`). Sharing
one filename recogniser between `init` and `new` dropped a length constraint, so
`2026-09-19-notes.md` parsed as record 2026 and the corpus was stuck above it
permanently, since numbers are never reused.

One stray record-shaped file no longer collapses every sibling record type into
its parent (`BUG-54`). A corpus with `docs/adr/`, `docs/rfc/` and a single
`docs/0099-index.md` proposed one meaningless `doc` type and ingested
`docs/README.md`, which is not a record.

`embodiment.locator-exists` says "is not a git-tracked file" rather than "is not
in the working tree" (`BUG-55`), which was false of a gitignored file.

A `claim_paths` entry that is not a readable directory now fails at startup
instead of silently disabling the rule (`BUG-56`).

`README.md` and `AGENTS.md` no longer instruct readers to edit
`.urzua/config.toml`, which this release stops reading.

#### `claim.status-agreement` no longer raises false errors on ordinary prose, a

malformed rule setting says what is wrong with it, and `off` is declarable on a
rule that takes options.

`claim.status-agreement` matched a verb as a substring and claimed every
reference on the line (`BUG-45`), so `prefixes` read as `fixes` and
*"Fixes BUG-39, which RFC-9 predicted"* raised a blocking error about `RFC-9`.
Verbs now match on word boundaries, and a claim binds to the references the
verb governs.

A malformed rule setting said only that data did not match an untagged enum
(`BUG-46`). `RuleSetting` now dispatches on the parsed value, so the messages
naming the valid levels and keys are reachable again.

A quoted reference in prose (`'RFC-9'`) is recognised, and the unused `toml`
workspace dependency is removed.

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
