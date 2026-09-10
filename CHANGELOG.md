# Changelog

All notable changes to `urzua` are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/). Written for the person installing it — what changed
for you — not a commit dump.

## 0.2.0 (2026-09-10)

### Breaking Changes

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

#### `fix --apply`/`new` now trust a verified `gh` login over `--by`

Identity resolution for `fix --apply` and `new` (RFC-2/ADR-31) now checks `gh api user` first,
then `git config user.name`, and only falls back to an explicit `--by` value when neither verified
source is available. Previously `--by` won outright, which meant a typed name could silently
override an authenticated `gh` session. If you were relying on `--by` overriding your `gh` login,
that no longer happens -- unset `GH_TOKEN`/log out of `gh` in that shell, or don't authenticate `gh`
in that environment. `gh api user` is also now bounded to 5 seconds rather than hanging indefinitely
if `gh` doesn't respond.

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

#### `pointer_fields`/`narrative_fields` are now config-declared per type

`pointer_resolution`, `header.pointer-field-clean`, and the renamed `narrative-field.stale` (was
`blocked-on.stale`) previously scanned a fixed Rust array — `Implements`/`Derives-from`/`Parent`/
`Blocked-on` — regardless of what a type's `.urzua/config.toml` entry actually declared. An org
adding a custom relationship field (e.g. `Feeds-into`) would pass `header.field-set-consistency`
but never get it resolved, checked for dangling references, or shown in `urzua graph`.

Both are now real per-type config keys, `pointer_fields` (clean, comma-separated references,
format-enforced) and `narrative_fields` (prose-tolerant, staleness-checked) — undeclared means zero
fields of that kind checked, no implicit default. Three new `check` rules enforce the declaration
itself: `config.pointer-declaration-missing` (a type must declare both lists explicitly, even as
`[]`), `config.pointer-field-not-known` (a declared field must also be in `required_fields`/
`known_fields`), and `config.pointer-narrative-overlap` (a field can't be in both lists).

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
