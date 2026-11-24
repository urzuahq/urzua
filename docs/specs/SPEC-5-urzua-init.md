---
Stable-Id: 01M6R3NK5PKKQRJF9WFZJA105B
Version: '0.34'
Date: 2026-08-20
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:5fca64adc779f68e6c32222b1d2a75ef83849903e92b9997f70bc7c739d76186, test:rust/crates/urzua-cli/tests/init_adopt_integration.rs@sha256:e4f6e92a173b18a55d8e9b40948e09e7f0590df6a0bee50c4ed02ab6c07a4ea7, code:rust/crates/urzua-cli/src/commands/init_greenfield.rs@sha256:b6fdfdbb365253afc460de143540443fd4b88be9da39e87d977bc7f7c88edcca, test:rust/crates/urzua-cli/tests/init_greenfield_integration.rs@sha256:98517eae0c2e6942f2fffe8dfda1991d89f3eb5054d069e9e57ed8ad7125f97f
Author: beauwilliams
Subject: '`urzua init` -- the adopt-existing-corpus path, greenfield type selection with built-in profiles, and the `.urzua/` layout.'
Implements: RFC-1
Parent: SPEC-1
---
# SPEC-5 — `urzua init`

## Purpose

`init` is where a repository acquires a governed corpus, and it is the first sixty seconds of the
product. It has two modes. Adopt infers which record types already exist from the tree and produces
the configuration every other command reads. Greenfield, for a repository with no records yet,
chooses the types to govern from a menu of built-in profiles (`--types`, or a prompt on a terminal)
and writes the profile's rules out as that same configuration.

Adopt is also where the multi-document-type position becomes visible for an existing corpus. Nearly
every tool in the landscape is ADR-only; RFC-1's core+profile model spans types by design, and adopt
proposes one type per directory it finds, whatever mix a repository already has.

**Adopt is the primary case regardless.** Every codebase this project was extracted from already had
records before it had tooling, and a tool whose setup path assumes an empty repository asks its most
likely user to migrate before they can evaluate it.

Adopt **never moves a file.** It reads the tree and proposes a config describing what is already
there. Restructuring is `migrate`'s job and carries a reverse-reference scan; folding that into `init`
would put a corpus-wide move behind a command whose name promises setup.

## Layout

Everything the tool owns lives under `.urzua/`, which is a directory rather than a root `urzua.yaml`
because more than config lives there (below) -- scattering config, templates and derived state across
the root, the corpus, and a cache directory is how each ends up governed by a different rule:

```
.urzua/
  config.yaml           tracked — the one config; written by init (shipped)
  templates/            tracked — a starting point per type, read by `new` (SPEC-12)
    adr.md                written by greenfield init for each selected type, or hand-authored
    rfc.md
  cache/                intended gitignored — derived, never authoritative; not yet used by any command
```

(`.gitignore` gaining `.urzua/cache/` is not yet built either -- `init` does not touch `.gitignore`
today, and criterion 2 below holds it to that.)

Adopt writes only `.urzua/config.yaml`. Greenfield writes `.urzua/config.yaml` and one
`.urzua/templates/<type>.md` per selected type, never over a template that already exists. `cache/` is
part of `.urzua/`'s settled shape but nothing in the write path generates it. Adopt creates no
directory. Greenfield creates each selected type's directory with an empty `.gitkeep`, so the directory the
config declares is in a fresh clone (git does not track an empty directory) and `doctor` and `check` hold on a
repository with no record yet; a directory that already exists is kept as it is, and a dry run creates none.
`new` still creates a type's directory when it is missing.

**Why templates leave the corpus.** This is the load-bearing reason, and it comes from this
repository. `docs/adr/_template.md` and `docs/rfc/_template.md` are git-tracked, sit inside the
record directories, and contain `Status: Proposed | Accepted | Rejected | Superseded`,
`Date: YYYY-MM-DD` and `# NNNN — Title`. Every one of those is invalid as a *record* and correct as
a *template*. A checker that discovers them reports errors on files that are exactly right.

The reflex is an ignore list. An ignore list is an ad-hoc exclusion that grows without bound, is
invisible to the reader who has the question, and is the same anti-pattern RFC-11 rejects for
boundaries and RFC-13 rejects for numbering gaps. Keeping templates out of the corpus removes the
question instead of suppressing it: the template is not a record, so it does not live where records
live, and no rule needs to know it exists.

`--config <file>` names the file `init` writes (and the one it refuses to overwrite); without it the file is `.urzua/config.yaml`. A default location is checked to resolve inside the repository, so a symlinked `.urzua` pointing outside is refused rather than written through.

`init` also proposes a directory whose files are not numbered records when at least two of its markdown files carry a header and they are at least half of the directory's markdown files (a flat spec directory named by slug), with no `prefix`, and says so in an `init-scan` notice. A proposed directory none of whose files has a header is named in an `init-scan` warning notice, since it may hold pages rather than records; it is not proposed (a type with nothing to read is noise), and the notice says to declare it by hand if it is a record directory. A hidden directory (any path component starting with a dot, such as `.changeset`) is never proposed or mentioned, a leading dot being neither a legal type name nor a legal prefix. An unnumbered directory needs, beyond the header test above, at least two headed files carrying a `status` field (any letter case); a directory of headed pages without one (front matter holding only `title`) is named in a notice and not proposed. A type name is lower-case ASCII letters and digits in hyphen-separated segments with no all-digit segment, so its upper-cased default prefix always loads; a directory whose records share a prefix an earlier directory already holds is not proposed and is named in a notice. The rendered config is parsed before anything is written, and one that does not load is refused.

## Type selection (greenfield)

```
urzua init                          # on a terminal, in a repository with no record directory: a menu
urzua init --types adr,rfc,spec     # non-interactive
urzua init --types adr --dir docs   # layout root
```

`--types` takes a comma-separated list of built-in profile names and selects greenfield mode whatever
the repository holds; `--dir` is the layout root, default `docs`, and each type lives in `<dir>/<type>`
(a relative path inside the repository, with no `..` and no hidden component). Without `--types`, `init`
runs adopt; only when adopt finds no record directory, the config does not exist and standard input is a
terminal does it show the menu on standard error and read the selection from standard input (numbers or
names). With no terminal it refuses exactly as adopt does, so a script never blocks on a prompt. `--dir`
without `--types`, when adopt would run, is refused rather than ignored. An unknown type name, a name
listed twice and an empty list are refused by name, with the available profiles, and nothing is written.

A profile is a **starting point that is written out, not a hidden default.** `init` materializes the
profile into `config.yaml` as an ordinary `record_types` entry (`dir`, `prefix`, `required_fields`,
`known_fields: []`, `header_shape`, `relation_fields`, `statuses`, and for `spec` a `revision_log` with
its version bump), and nothing in the config names a profile. A user who disagrees with a required
field edits a visible line. The `rules` section proposes every rule adopt proposes, at `warn`, and also
those that read `required_fields` or `known_fields` (`field.quality`, `field.pending`,
`header.field-set-consistency`), which the profile now declares; the rest are withheld with the same
`rule-applicability` notice as adopt. The `authority` and `harness` keys are the same as adopt's.

The built-in profiles:

| Profile | Required fields | Statuses (first is the one `new` writes) | Lifecycle |
|---|---|---|---|
| `adr` | `Status`, `Date`, `Author`, `Deciders`, `Stable-Id` | `Proposed`, `Accepted`, `Rejected`, `Superseded` | `Proposed` to `Accepted` or `Rejected`; `Accepted` to `Superseded` |
| `rfc` | `Status`, `Date`, `Author`, `Stable-Id` | `Draft`, `Discussion`, `Accepted`, `Rejected`, `Superseded` | `Draft` to `Discussion`, `Accepted` or `Rejected`; `Discussion` to `Accepted` or `Rejected`; `Accepted` to `Superseded` |
| `spec` | `Status`, `Date`, `Version`, `Author`, `Subject`, `Stable-Id` | `Draft`, `Accepted`, `Shipped` | `Draft` to `Accepted` to `Shipped` |
| `prd` | `Status`, `Date`, `Author`, `Owner`, `Stable-Id` | `Draft`, `Review`, `Approved`, `Rejected`, `Superseded` | `Draft` to `Review`; `Review` to `Approved` or `Rejected`; `Approved` to `Superseded` |

The role each type is accountable to is a required field: `Deciders` on an `adr`, `Owner` on a `prd`,
`Author` elsewhere. Every profile requires `Stable-Id`, which `new` always assigns (`ADR-21`). The
`prd` profile is provisional: it answers `RFC-1`'s open question of whether a PRD fits the core+profile
model by shipping a profile of the same shape, and it is revisited if that RFC decides otherwise.

Each selected type gets a starter template, `.urzua/templates/<type>.md`, whose header names the
profile's fields and whose body is the type's section scaffold. `new` reads it (`SPEC-12`). A template
that exists is kept as it is and reported `kept`.

Greenfield writes the templates first and the config last, whole or not at all, so a config that
exists is complete. The rendered config is parsed before anything is written. `--dry-run` prints the
plan and writes nothing, and its `config_yaml` is byte-identical to the file a real run writes.

**Re-running.** A run whose `--types` and `--dir` would write exactly the config that exists is a no-op
that exits 0 with `written: false` (missing templates are still written). Any other run against an
existing config is refused, exit 2, as in adopt.

**Unclassified files.** A record-shaped, non-hidden file outside every selected type's directory is
named by path in an `init-scan` warning notice, never an error: nothing governs it until a type is
declared for its directory. Adopt reports the one kind of record-shaped file it cannot classify by
path: a file at the repository root, which has no directory to name a type, in an `init-scan` info
notice. (A directory adopt declines to propose is already named in its own notice, and a hidden
directory is never mentioned.) Neither mode changes its exit code for these.

## Proposed header shape

`init` proposes `header_shape: yaml-frontmatter` for every type whose records carry a header (a type
whose records carry none is the next section), the only shape with a reader (the
`blockquote` and `bold-list` shapes were removed, `MILE-85`). For each proposed type it parses the
existing records with the real header parser and counts those whose header is YAML frontmatter at the
very start of the record; a record written in a removed form, or with no readable header, is counted
in a `header-shape` notice that names the form (`2 in the removed bold-list form`), which no rule reads
until its header is rewritten as YAML frontmatter. The proposed config declares no `required_fields`, so
`check` does not mention such a record; the notice says so and names the fix, declaring a field every
record carries (`required_fields: [Status]`) on the type, after which `header.required-fields` reports
each one. A type whose records are all frontmatter gets no notice. A type with no records is proposed
`yaml-frontmatter`.
Each `proposed` entry reports the chosen `header_shape`. A file directly in a proposed directory that is
not a record is written under the type's `exclude_files` and named in the entry's `notes`: an index page
(`README.md` or `index.md`, any letter case, written as spelled) and, only in a directory with no header,
a markdown file with no number and no header beside numbered ones. A numberless file with a readable
header stays a record, so `filename.identity-pattern` reports its name. A directory with none writes no
`exclude_files`.

A directory where only some numbered files carry a header keeps `yaml-frontmatter` with its
`header-shape` notice, and also gets the `sections.required` its files' bodies share, by the threshold
below; its `notes` say how many files have no header. `filename.identity-pattern`'s
rename guidance for a file with no number names `exclude_files` as the alternative (`SPEC-2`).

## A corpus whose records have no header

A numbered directory none of whose files has a header block (the common ADR style: `0001-title.md`, a
`# 1. Title` heading, a `Date:` line and `## Status`/`## Context`/`## Decision`/`## Consequences`
sections) is proposed with `header_shape: none` when at least two of its files share a `##` heading
carried by at least 80% of them, and the directory holds decision records by one of two signs: a shared
heading from the built-in vocabulary `Status`, `Context`, `Decision`, `Consequences`, `Decision Outcome`,
`Context and Problem Statement`, `Decision Drivers`, `Considered Options`, or a directory named `adr`,
`adrs`, `decisions`, `rfc` or `rfcs`. Headings are compared without letter case or a trailing colon:
`Decision`, `Decision:` and `decision` count as one heading for the threshold, the most common spelling is
written, and the notes name the other spellings and say `section.required` reports those files until they
are respelled or the spelling is added as an alias. The required headings, in the corpus's own order, are
written as `sections.required` (`from: headings`) and listed in the entry's `required_sections`, so
`section.required` reports a file that lacks one.

A headerless directory that is not proposed is named in an `init-scan` notice with its reason: a single
numbered file, no `##` heading in any file (headings at another level are not sections), or no heading
carried by 80% of the files ("hold pages rather than records"). One whose files share headings outside the
vocabulary, under a name outside the list, is named as looking like ordinary pages (numbered tutorial
pages are the case), with the way to adopt it anyway: declare it under `record_types` with `header_shape:
none` and `sections.required`. `init` has no flag to force a directory. The vocabulary is built in, not
configured, because `init` runs before any configuration exists to declare one in (the `ADR-101` amendment
of 2026-11-15 puts vocabularies in configuration); no flag overrides it, since a corpus outside it is
adopted by declaring its type, which is the configuration the vocabulary would otherwise live in.

The entry's `notes` say what the proposal observed and what it cannot express: the threshold and the
headings that met it, each required heading's missing files by path, the headings below the threshold,
that no field is read (a `Status:` or `Date:` line in the body included), that a `## Status` section is
checked for presence only and not read as a status (`MILE-255`), that a reference written in prose (`ADR
0007`) is not read as a reference (`MILE-268`), and, for file names with no type prefix, the number's
width and padding, that urzua gives no identity to such a name (`ADR-36`; an `identity.pattern` must
capture a prefix), and every number more than one file carries, which `check` cannot report under the
proposed config. No prefix is invented. The threshold and the section rule were decided by the builder
on 2026-11-16, pending maintainer review (`MILE-265`).

A directory where some files carry a header and some do not stays `yaml-frontmatter` with its
`header-shape` notice. In a mixed corpus each directory is proposed by what it holds.

## Proposed harness key

`init` also proposes `harness: {outside_writes: shared}`. It enables no guardrail; it makes a guardrail the
adopter turns on later also judge the files of a linked worktree of the repository (`ADR-89` amendment of
2026-11-09, outside writes; `SPEC-3`).

## The waiver declaration is withheld, not proposed

A waiver type makes records suppress findings, so it is the adopter's to opt into: `init` infers no `role` from a directory's name and proposes no `role: suppression` type. The `rule-applicability` notice that withholds `waiver.matches-nothing` names what to declare: a record type with `role: suppression` and `effective_requires: [Reason, Expires]` (or `permanent`), the form `waive --from-report` accepts. `harness.outside_writes` is proposed, as above, because alone it enables nothing.

## Proposed rules

`init` proposes each shipped rule at `warn`, except a rule that cannot examine anything on the corpus it
just adopted or on the one greenfield writes: a rule that needs options it cannot supply, a rule that needs declared `relation_pairs`,
a rule that resolves identity on a corpus where no type has a prefix, a rule that reads a per-type
declaration `init` does not write, `identity.collision` and `identity.open-pr-collision` where no
proposed type has a prefix (each reads a record's number from the file name's prefix), and, where no
proposed type has a header, every rule whose table entry is `header_none: OutOfScope`; and
`header.scaffold-mismatch` (`MILE-103`), which asks whether a
record was written with `urzua new` and which no record of a just-adopted corpus was, so it would warn
on the first record each author adds by hand; an adopter turns it on once they use `new`. Unlike the
others it is named in `init`, not derived from the rule table, and no notice lists it. The last group is not a fixed list: it is every rule whose entry in the
rule table carries a `reads_declaration` (`required_fields`, `known_fields`, `required_fields` or `known_fields`
for the field-hygiene rules, `pointer_fields` or `narrative_fields`, a supersession field and a locator field
listed in `known_fields`, `revision_log`, `revision_log.version_bump`, `statuses.values`, `statuses.require`,
`statuses.forbid`, `statuses.transitions`, `statuses.delivery`, `statuses.blocked` or `statuses.progress`, `statuses.delivery.items`, `sections.required`, and the three section-structure keys `sections.unique_headings`, `sections.no_skipped_levels` and `sections.order`), so a
rule added later with a `reads_declaration` is withheld without touching `init`. The annotation is audited
on the output: `init_integration.rs` and `init_greenfield_integration.rs` each run `check` on a proposed
config and fail when a proposed rule reports `ran` over zero records. Adopt writes
`required_fields: []` and infers none of those declarations (`ADR-33`); greenfield writes the declarations
its profiles carry (fields, status values and transitions, and for `spec` the revision log and its
`version_bump`), so it proposes the rules that read those, and withholds the rest. Greenfield also withholds
`type.no-declared-spec`, which would warn about the `spec` pointer of every type it just wrote while no spec
exists yet; the notice names it, and the rule is enabled once a type declares `spec: SPEC-N`. A `rule-applicability` notice
(severity `info`) lists the withheld rules, grouped by declaration, so the adopter knows what to declare on
a record type before enabling the rule. Adopt writes `sections.required` only for a headerless type, so
then it proposes `section.required` and still withholds `section.shape`, which judges only an entry with
a `shape`; further `rule-applicability` notices name `section.shape`, the prefix-reading rules and the
header-reading rules it withheld for those reasons. Adopt also withholds
`config.known-fields-declaration-missing`, which asks every headed type to declare `known_fields` and so
would warn about the config adopt just wrote; a notice names it. The first `check` after `init` reports
no `config.*` finding about the proposed config on the headerless and mixed corpora the tests hold: a type
declared `header_shape: none` and a file with no identity are not counted by `config.scope-matches-nothing`
(`SPEC-2`).

`init` also writes `authority: {decider_table: base, waivable: never}`: no deciders and no gates, which no
corpus can supply, only the choice that who may decide is read from a pull request's base revision (`SPEC-3`,
`ADR-89` amendment of 2026-11-09), so the table the adopter writes next is not one a pull request can edit for
itself, and the choice that `diff.decider-required` is never waivable (`ADR-89` amendment of 2026-11-09, gate
waivers).

## Safety

- **Never clobber.** An existing `.urzua/config.yaml` is not overwritten. `init` reports what exists
  and exits 2. `init --dry-run` still runs, shows what adopt would propose today with a notice that
  a config exists, and writes nothing.
- **Idempotent, via the never-clobber refusal above.** A re-run (without `--dry-run`) against an
  initialized repository exits 2 and changes nothing.
- **The config is written whole or not at all** (a temporary file renamed into place), so a full disk
  or a killed process leaves no truncated `.urzua/config.yaml`, which a re-run would refuse to
  overwrite.
- **`--dry-run` prints the plan and writes nothing**, and is the documented way to see what adopt
  inferred before committing to it.

## Output (shipped)

The real, shipped shape (`InitReport`/ADR-46):

```json
{
  "status": "ok",
  "dry_run": false,
  "config_path": ".urzua/config.yaml",
  "proposed": [{ "name": "adr", "dir": "docs/adr", "record_count": 5, "prefix": null, "header_shape": "yaml-frontmatter" }],
  "written": true
}
```

An entry also carries `exclude_files`, `required_sections` and `notes` when it has any.

When adopt finds no record directory the refusal names greenfield mode: its `error` ends with the
exact command, `urzua init --types adr`, and the built-in profiles, and its top-level `next_action` is
that command (`SPEC-2`). `urzua doctor` with no config states what `init` would do there today: the
number of types adopt proposes (`next_action: urzua init`), greenfield's command when there is nothing
to adopt, or adopt's reason when it cannot write a config (`next_action: null`); it never sends the
user to an `init` that refuses.

Greenfield mode adds `templates`, a list of `{type, path, state}` where `state` is `written`, `kept` or
`would-write`, and `directories`, a list of `{type, path, state}` for each type's record directory with the same
three states; both are absent in adopt. A greenfield `proposed` entry carries its profile's `prefix`, and
its `record_count` counts the record-shaped files already in its directory.

`config_yaml` (the full rendered config) is present only when `dry_run: true` -- there's nothing
else to read the preview from, since nothing was written. A real write omits it, matching `new`'s own
established convention (SPEC-12): the caller reads the file at `config_path` if it needs the content.

## Exit codes (shipped)

| Code | When |
|---|---|
| 0 | proposal computed (dry-run), config written, or a greenfield re-run that would write the config that exists |
| 2 | refused via `CouldNotRun` -- existing config (a greenfield re-run that differs), no record-shaped files found, unknown or repeated `--types` name, a `--dir` that is not a relative path inside the repository, `--dir` without `--types` in adopt, unwritable path |

There is no exit `1`: a file `init` could not classify is a notice and the exit stays `0` (maintainer
decision, 2026-11-13).

## Success criteria

1. `--dry-run` output matches what a real run then does, byte for byte.
2. Adopt moves no files, and a `git status` after it shows only `.urzua/`; `init` does not touch `.gitignore`.

Criterion 1 is enforced by `init_adopt_integration.rs` (`init_dry_run_proposes_the_config_the_real_run_writes`),
criterion 2 by `init_integration.rs` (`init_changes_only_the_urzua_directory_and_leaves_gitignore_alone`), and
the withheld-rule mechanism by the same file (`init_withholds_every_rule_that_reads_a_declaration`).
Criterion 2 is the one to hold. The moment `init` moves a file it becomes a migration, and a
migration without a reverse-reference scan is the documented data-loss shape.

3. A `urzua init --types adr,rfc,spec` run produces a config `check` accepts, with record-shaped files
   outside the selected directories reported rather than errored. On an empty repository `init`, `doctor`
   and `check` each exit 0 and `check` reports no finding (an `ok` run over no record, with an `empty-corpus`
   notice).
4. A greenfield re-run that would write the config that exists exits 0 and changes nothing.

Criteria 3 and 4 are enforced by `init_greenfield_integration.rs`
(`the_written_config_passes_check_and_new_writes_a_stable_id_for_every_type`,
`on_an_empty_repository_init_doctor_and_check_all_pass_with_zero_findings`,
`record_shaped_files_outside_the_selected_types_are_reported_not_errored` and
`a_rerun_with_the_same_selection_is_a_no_op_and_a_different_one_is_refused`).

## Open questions

None open. The four questions `MILE-114` carried are decided or recorded:

- **CI and hook wiring.** Opt-in, never default: `init` writes nothing outside `.urzua/`. A `--with-ci`
  flag is the shape it would take and is not built.
- **The display-number scheme.** Not chosen by `init`. `ADR-3` separates the stable identifier from the
  display number, and `new` numbers each type's records sequentially from its existing filenames; the
  prefix a greenfield profile writes is the type name upper-cased.
- **Whether adopt infers rules or only structure.** Structure only (`ADR-33`): adopt writes
  `required_fields: []` and infers no statuses; greenfield is the way to start from rules.
- **One `.urzua/` per repository, for a monorepo.** One, at the repository root: independent corpora
  are separate types, each with its own `dir`. A `.urzua/` per subtree is not built. `RFC-11` section 6,
  on scope, does not answer this and stays open.

## References

- SPEC-2 — `check`, the consumer of what this writes.
- SPEC-3 — configuration; amended by this spec to live at `.urzua/config.yaml`.
- RFC-1 — core+profile, which type selection instantiates.
- RFC-11 — why an ignore list is the wrong answer to the template problem.
- MILE-114 — greenfield mode, `--types`/`--dir`, built-in profiles, and unclassified-file reporting,
  relocated from this spec's own Draft sections (`BUG-26`) and built.
- ADR-3 — stable identifiers, separate from display numbers.
- ADR-21 — every type gets a `Stable-Id`, which every built-in profile requires.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Initial spec. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Added `## Output (shipped)`/`## Exit codes (shipped)` documenting the real, narrower `InitReport` shape (BUG-20, ADR-46), separated from the Draft's own richer aspirational design (greenfield mode, `--types`/`--dir`, `unclassified` tracking, exit 1) so a reader can tell which parts are built. | **substantive** |
> | 2026-09-17 | Config moves from `.urzua/config.toml` to `.urzua/config.yaml` (`ADR-52`, shipped in the same change). The described mechanism changes, not just its rendering. | **substantive** |
> | 2026-09-18 | Stays `Draft`, deliberately, and is the genuinely mixed case `BUG-26` identified. **Why:** the adopt path is built and tested -- `BUG-36`/`BUG-37` closed yesterday and `init` now adopts a real foreign corpus end to end. The rest is not: `init`'s only command-specific flag is `--dry-run` (`--config` is global, declared `#[arg(long, global = true)]`), so §"Type selection"'s `--types adr,rfc,spec`, its built-in `adr`/`rfc`/`spec`/`prd` profiles and its interactive mode do not exist, §Safety claims a re-run exits 0 when it exits 2, and §Success criteria depends on unclassified-file reporting that was never built. Flipping it would make the spec assert commands that error; leaving it `Draft` understates the half that ships. The aspirational sections need to move to a record that can hold unbuilt design before the flip -- that move is the remaining work on `BUG-26`. | **structural** |
> | 2026-09-24 | Bumped to `0.5`, flipped to `Accepted` with `Embodiment: Verified`, closing `BUG-26`. **Why:** relocated every aspirational section (greenfield mode, `--types`/`--dir`, built-in profiles, unclassified-file reporting, the two unbuilt success criteria, all four open questions) to the new `MILE-114`, then re-verified every remaining claim against the binary rather than assuming the previous Draft/shipped split was already complete. Found two more inaccuracies the same pass: the Layout diagram implied `init` writes `templates/`/`cache/`/`.gitignore`, none of which it touches -- only `.urzua/config.yaml` is written today; corrected rather than carried forward silently. | **substantive** |
> | 2026-11-02 | Bumped to `0.6`. **Why:** added `## Proposed header shape` and the `header_shape` field on each `proposed` entry. Adopt mode proposes the shape each type's records already use instead of `yaml-frontmatter` for every type (`BUG-151`, `ADR-33` amended), because the previous behavior produced a config that read none of a real adopter corpus. | **substantive** |
> | 2026-11-02 | Leading YAML frontmatter that parses with fields takes precedence over the field-count rule. **Why:** review found body `- **Key:**` bullets could outvote a real frontmatter header. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.7`. `init` proposes only `yaml-frontmatter` and reports, in a `header-shape` notice, the existing records written in a removed `blockquote` or `bold-list` form; it no longer infers a shape from the corpus and no longer declares `header.deprecated-shape`. **Why:** `MILE-85` removed those shapes. | **substantive** |
> | 2026-11-04 | Bumped to `0.8`. Added `## Proposed rules`: `init` does not propose a rule that reads a per-type declaration it leaves empty (`field.quality`, `field.pending`, `header.field-set-consistency`, `header.pointer-field-clean`) and names each in a `rule-applicability` notice with the declaration to write. **Why:** `BUG-73`; the four rules ran over zero records on every adopted corpus. | **substantive** |
> | 2026-11-05 | Bumped to `0.9`. `init` writes the config through a temporary file and a rename. **Why:** review of the CLI found every record and config write truncated the file before writing it. | **substantive** |
> | 2026-11-05 | Bumped to `0.10`. `init` proposes a header-bearing directory of unnumbered files and flags a proposed directory with no header in any file, in `init-scan` notices. **Why:** Phase 0 hardening from an adopter switchover -- the spec directory was missed and two directories of pages were proposed without comment. | **substantive** |
> | 2026-11-06 | Bumped to `0.11`. `init` never proposes a hidden directory or a directory none of whose files has a header, proposes an unnumbered directory only when at least two of its headed files carry a `status` field, legalises every type name, drops a type whose prefix an earlier one holds, refuses to write a config that does not load, and `--dry-run` runs when a config already exists. **Why:** Phase 0 hardening from a second adopter switchover -- `init` wrote a config naming a `.changeset` type that could not load, and proposed 39 types. | **substantive** |
> | 2026-11-07 | Bumped to `0.12`. `init` writes the file named by the global `--config`, and checks the default location resolves inside the repository. **Why:** a Phase 0 hardening review found `--config X init` reported `written: true` for a file it never created, writing `.urzua/config.yaml` instead. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: init.rs and check_integration.rs still implement and test urzua init adopt mode (SPEC-5), status_model.rs, the own-config test and status_delivery_integration.rs still implement the milestone type and its delivery status (SPEC-6), both fix.rs files and the paths, rebaseline and drift integration tests still implement urzua fix (SPEC-8), and config.yaml and the own-config tests still declare the bug type (SPEC-9). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082148a47756ba8d8a4b6e8dc7d85f66edb3748c63e7a7e5f01c96c, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b8337c1172d3c4825859e00a0344dd4c35e6f013d802e509b1688ecb` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-08 | Bumped to `0.13`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` efe3f3739082 -> 08dfadd3cae6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. init.rs: only the doc comment on run changed, to say the config a dry run proposes is byte-identical to the file the real run writes and that the reports differ in dry_run, written and the echoed config_yaml; no code changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.14`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Bumped to `0.15`. The `header-shape` notice states what `check` does (silent while `required_fields` is empty) and the config that makes it report, instead of promising a report that never came; `init` writes `exclude_files: [README.md]` for a proposed directory that holds a README. The written config for a corpus with no README, and every existing config, are unchanged. **Why:** the round-4 adopter review (S3, N1): the warning was not true, and every adopted corpus drew `filename.identity-pattern` warnings asking to rename its README. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 08dfadd3cae6 -> 5f376bdd1b6d, `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged; init.rs now words the header-shape notice truthfully (check reports unreadable headers only when required_fields declares them), excludes README.md for a directory holding one, and carries the merged doc-comment edit from main; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 5f376bdd1b6d -> 6c671b5f6ed7. Reason: split init::run into propose_record_types and write_config, and render_config_yaml into proposed_types_mapping and proposed_rules_mapping (init.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 6c671b5f6ed7 -> eab52da49062. Reason: Read the diff of each re-pinned file against main. cli/commands/init.rs: Notice literals use Notice::new or gain detail: None; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.18`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.19`. Adds `## Proposed harness key`: the proposed config declares `harness.outside_writes: shared`. **Why:** `ADR-89` amendment of 2026-11-09 (outside writes), accepted by the maintainer: a linked worktree at a commit without gates was an unjudged place to write. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` eab52da49062 -> 1520764ba016. Reason: init.rs: read the diff since the previous pin; render_config_yaml adds a top-level harness mapping {outside_writes: shared} to the proposed config (6 added lines, no other change to the file). This record's claim about init is unaffected; SPEC-5 0.19 documents the new key. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.20`. The proposed configuration carries `authority: {decider_table: base}`, with no deciders or gates. **Why:** `ADR-89` amendment of 2026-11-09 (decider table), accepted by the maintainer. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 1520764ba016 -> ee569c2dd2ff. Reason: Read the diff of each pinned file since the old pin and the claim still holds. init.rs: render_config_yaml adds an authority mapping holding only decider_table: base to the proposed configuration; nothing else in the pinned file changed. Behaviour unchanged for a repository that does not declare decider_table. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.21`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` ee569c2dd2ff -> 1cb2edb3e555. Reason: Read the diff of init.rs: the authority block `urzua init` writes gains `waivable: never`; nothing else changed. The pinned evidence still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.22`. Evidence pins on `.urzua/config.yaml` were re-pinned (row above) after the config's authority section gained `waivable: never`; the specification's content is unchanged. **Why:** `ADR-89` amendment of 2026-11-09 (gate waivers). | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 5ab61b95fd58 -> a60c9c61b94c. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/tests/check_integration.rs: adds one regression test that a claim file with an upper-case .MD extension under claim_paths is read; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.23`. Evidence pins re-baselined for this change; the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-13 | Bumped to `0.24`. `## Proposed rules` states the withheld-rule mechanism (`reads_declaration` in the rule table) and no longer a fixed list of four, which had fallen behind the table; success criterion 2 no longer claims `.gitignore` appears in `git status`, because `init` never touches it. `Realized-by` cites `init_adopt_integration.rs` instead of `check_integration.rs`, and `init_integration.rs` carries a `Verifies: SPEC-5` back-pointer; together they enforce both criteria and the mechanism. **Why:** a re-read against the code found these divergences. | **substantive** |
> | 2026-11-13 | Bumped to `0.25`. Added `## Type selection (greenfield)`: `--types` and `--dir`, the built-in `adr`, `rfc`, `spec` and `prd` profiles written out into `config.yaml`, starter templates, the interactive menu, the greenfield re-run, and the report of record-shaped files no type governs; adopt reports a record at the repository root in an `init-scan` notice; the output gains `templates`; the four open questions are decided or recorded. **Why:** `MILE-114`. | **substantive** |
> | 2026-11-13 | Maintainer decision, 2026-11-13: the profile contents above are accepted, with the lifecycle transitions of `adr` and `prd` as decided (`rfc` and `spec` get the forward-only graph the engine requires beside `initial`, a choice the decision did not make, open to correction) and with `Stable-Id` required in every profile; `prd` ships as provisional, answering `RFC-1`'s PRD question; starter templates are written by default; a file `init` could not classify is a notice and never changes the exit code, so adopt keeps its behaviour, which `adopt_config_is_byte_identical_to_the_pre_greenfield_output` pins to the config the build before greenfield mode wrote. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 1cb2edb3e555 -> 92b8fdb24b1b. Reason: Read the diff of each pinned file since the old pin and the claim still holds. init.rs: adopt also names a record at the repository root in an init-scan notice, and the config assembly and writer are now shared with the new greenfield mode in init_greenfield.rs (run takes an InitArgs); SPEC-5 was rewritten in this change to describe greenfield mode, its profiles and the decided open questions, and still describes adopt. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/tests/init_adopt_integration.rs@sha256:aa1cfba9b1e4, rust/crates/urzua-cli/src/commands/init_greenfield.rs@sha256:bbf43c454ad0, rust/crates/urzua-cli/tests/init_greenfield_integration.rs@sha256:b64108a3c965 Reason: Pinned the new locators as they are now after checking each against the record it evidences: init_greenfield.rs and init_greenfield_integration.rs are the greenfield mode this change adds and the tests that exercise it, and each test or function named in a Done means item asserts the behaviour the item states. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/init_greenfield_integration.rs` b64108a3c965 -> 58439be42591. Reason: Read the diff of the pinned file since the pin and the claim still holds. init_greenfield_integration.rs: the terminal-menu test was restructured so its test locator resolves (a bare block and a brace literal removed) with the same assertions; no behaviour tested changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/init_adopt_integration.rs` aa1cfba9b1e4 -> 171fdc60fb17, `rust/crates/urzua-cli/src/commands/init_greenfield.rs` bbf43c454ad0 -> c82169d04f38, `rust/crates/urzua-cli/tests/init_greenfield_integration.rs` 58439be42591 -> 54622fb732a2. Reason: init_greenfield.rs is a new file against main holding the greenfield profiles, config writer, templates and menu that SPEC-5 now describes; init_adopt_integration.rs: against main it adds one test, adopt_config_is_byte_identical_to_the_pre_greenfield_output, comparing adopt's config to a fixture generated by the pre-greenfield build, and nothing else changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/init_greenfield_integration.rs` 54622fb732a2 -> 3b3a5a29b77c. Reason: init_greenfield_integration.rs is a new file against main; the terminal-menu test now redirects the binary's stdout to a file inside the pseudo-terminal and asserts the menu is on the terminal only and stdout is the JSON report alone. The claim each record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.26`. `init` no longer proposes `section.duplicate`, `section.level-skipped` and `section.order`, which each read a `sections.*` key it never writes, and the withheld-rule notice names each with its key. The rest of the adopted config is unchanged. **Why:** since `MILE-104` they were proposed at `warn` and reported `ran` over zero records, the silent no-op `BUG-73` withheld the other declaration-reading rules for. | **substantive** |
> | 2026-11-13 | Maintainer decision, 2026-11-13: `section.required` and `section.shape` are withheld the same way, naming `sections.required`. Both read it, `init` never writes it, and in an adopted repo they reported `ran` over zero records. The adopted config no longer carries those two rules; a repo that declares `sections.required` enables them as before. **Why:** the same silent no-op `BUG-73` withheld the other declaration-reading rules for; the section-structure rows above had left these two as the remaining cases. | **substantive** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 92b8fdb24b1b -> 968559aeb15f, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` 171fdc60fb17 -> d5e0c9b6c2eb. Reason: Read the diff of each re-pinned file against main and the claim still holds. init.rs: the unit test the_withheld_rules_are_disclosed_with_the_declaration_each_needs also expects section.duplicate, section.level-skipped and section.order with their keys; no non-test code changed; init_adopt_integration.rs: adds adopt_withholds_the_section_structure_rules_and_names_the_key_each_needs; no existing test changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 968559aeb15f -> 15df5856d881, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` d5e0c9b6c2eb -> 8a65e3417c62. Reason: Read the diff of each re-pinned file against main and the claim still holds. init.rs: the unit test the_withheld_rules_are_disclosed_with_the_declaration_each_needs also expects the section rules with their keys; no non-test code changed. init_adopt_integration.rs: adds adopt_withholds_the_section_structure_rules_and_names_the_key_each_needs, which asserts the five section rules are not written and the notice names each with its key; no existing test changed. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-14 | Bumped to `0.27`. `## Proposed rules`'s statement of the `authority` block `init` writes now includes `waivable: never`, which `init` has written since 2026-11-09 (the `0.22` row recorded the config change but left this sentence stale). **Why:** the docs-versus-binary review of 2026-11-14 found the sentence disagreeing with `init`'s output. | **substantive** |
> | 2026-11-14 | Bumped to `0.28`. `## Proposed rules` names `header.scaffold-mismatch` as a rule `init` does not propose: it judges a record against what `urzua new` writes, which no record of a corpus just adopted was, so proposing it would warn on every hand-added record. `init`'s output for an existing corpus is unchanged from before the rule existed. **Why:** `MILE-103`; `init` proposes every shipped rule at `warn` unless one of the listed reasons applies, and a new opt-in rule would otherwise have been proposed. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 15df5856d881 -> 795a4f0f1c3e. Reason: Read the diff of each re-pinned file against main and the claim still holds. rust/crates/urzua-cli/src/commands/init.rs: proposed_rules_mapping skips header.scaffold-mismatch (a new early continue with a comment) and a unit test asserts adopt does not propose it; nothing else changed. rust/crates/urzua-cli/tests/suggested_action_integration.rs#rules_lists_each_variant_with_its_action_or_its_exemption_reason: the expected number of shipped rules is 74, not 73, because the rule table gained header.scaffold-mismatch; the assertions are otherwise unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.29`. Greenfield creates each selected type's directory with a `.gitkeep` and reports `directories`; it proposes the rules that read the declarations its profiles write (including `revision-log.version-bump-required` with the `spec` profile) and withholds `type.no-declared-spec`. Every rule that reads a per-type declaration, now audited over all rules, is withheld from adopt: `header.required-fields`, `pointer.resolution`, the field-hygiene, status, supersession, locator, revision-log and waiver rules join the earlier ones. Criterion 3 states the empty-repository case. **Why:** the first-run review, findings 6.1, 6.2 and 6.5: `init`'s output failed `doctor` and `check` on an empty repository, fifteen adopted rules reported `ran` over zero records, and the `spec` profile's version bump was withheld though declared. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 795a4f0f1c3e -> 31e687bccdc4, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` 8a65e3417c62 -> f7562c0496e1, `rust/crates/urzua-cli/src/commands/init_greenfield.rs` c82169d04f38 -> 5ef5edc7661c, `rust/crates/urzua-cli/tests/init_greenfield_integration.rs` 3b3a5a29b77c -> 39ddaa9e8476. Reason: Read the diff of each pinned file against main and the claim still holds. init.rs: InitReport gains a directories list (empty for adopt), the old-form warning says to enable header.required-fields after declaring required_fields, and the unit tests expect the newly withheld rules; adopt's rendering code is unchanged; init_greenfield.rs: greenfield creates each type's directory with a .gitkeep (write_directories, DirectoryOutcome), computes the declared set from the selected profiles (declared) so spec proposes the version-bump rule, and withholds type.no-declared-spec; profiles, templates and the menu are unchanged; init_adopt_integration.rs: the old-form test enables header.required-fields after declaring required_fields, and its comment says why; the golden test is unchanged and reads the regenerated adopt config; init_greenfield_integration.rs: new tests for the empty repository, directories, the spec version-bump rule and zero-examined rules; the greenfield-touches test allows the .gitkeep files; the scaffold test asserts exit 0 and non-blocking; other tests are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 31e687bccdc4 -> 3b79d9cdc769. Reason: Pinned file(s) changed by the symlinked-record refusal: init.rs: init proposes no entry for a rule that is on by default. The record's claim is unaffected by these changes. (Re-applied after merging main.) **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.30`. A pinned evidence file was re-baselined for the symlinked-record refusal (`ADR-89` amendment of 2026-11-14); this spec's text is unchanged. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/init_greenfield_integration.rs` 39ddaa9e8476 -> 98517eae0c2e. Reason: Read the diff of each re-pinned file against main: init_greenfield_integration.rs leaves rules on for every configuration out of the every-rule-examines-a-record test. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.31`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-14 | Bumped to `0.32`. States that the waiver declaration is withheld rather than proposed, and that the withheld notice names `role: suppression` with `effective_requires: [Reason, Expires]` or `permanent`. **Why:** the adopter switchover (items 3 and 13): `init` proposed nothing for waivers and `waive --from-report` then refused the hand-written type; `harness.outside_writes` was already proposed. | **substantive** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 31e687bccdc4 -> 77c67b7da3c6, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` f7562c0496e1 -> 11f72d46590a. Reason: Read the diff of each re-pinned file against main. init.rs changed only in its unit test, which now also asserts that the withheld waiver declaration names effective_requires; no production code changed; a test was added for the withheld waiver declaration; the existing tests are unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 3b79d9cdc769 -> 2a3bec5a6752, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` f7562c0496e1 -> 11f72d46590a. Reason: Read the diff of each re-pinned file against main. init.rs: only its unit test changed (one added needle, effective_requires: [Reason, Expires]); no production code changed; init_adopt_integration.rs: one test added (init_withholds_the_waiver_declaration_and_names_its_keys); no existing test changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 3b79d9cdc769 -> 2a3bec5a6752, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` f7562c0496e1 -> 11f72d46590a. Reason: Read the diff of each re-pinned file against main. init.rs: only its unit test changed (one added needle, effective_requires: [Reason, Expires]); no production code changed; init_adopt_integration.rs: one test added (init_withholds_the_waiver_declaration_and_names_its_keys); no existing test changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Bumped to `0.34`. Adopt proposes a numbered directory whose files carry no header as `header_shape: none` with the `##` headings at least 80% of its files share as `sections.required`, names in `notes` what it cannot express (missing sections by file, duplicate numbers, the unread `## Status` and prose references, no identity without a prefix), withholds the header-reading rules when no type has a header and the prefix-reading collision rules when no type has a prefix, excludes `index.md` and numberless files beside numbered records, and names `urzua init --types adr` as the next step when there is nothing to adopt; `doctor` names a working step. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** `MILE-265` -- a walkthrough of 2026-11-15 found `init` refusing the common ADR style and `doctor` sending the user back to it. | **substantive** |
> | 2026-11-16 | Bumped to `0.33`. The pins re-baselined above follow code changed for `MILE-266` and `MILE-267` (printed next actions that run as printed, a revision log in every built-in template); nothing this spec states changes. **Why:** the re-pin rows are structural and `revision-log.version-bump-required` asks for a bump. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 2a3bec5a6752 -> ded385eeb996, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` 11f72d46590a -> 566bb95965e7, `rust/crates/urzua-cli/src/commands/init_greenfield.rs` 5ef5edc7661c -> 15c7e3815148. Reason: init.rs: adopt proposes header_shape none with sections.required for a numbered headerless directory, excludes index.md and numberless files, and withholds rules that would examine nothing there; init_greenfield.rs: the greenfield proposal initializer gains empty required_sections and notes fields; greenfield output is unchanged; init_adopt_integration.rs: adds tests for the headerless corpus, mixed corpus, index exclusions, rename hint and migrate ids; existing tests are unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init_greenfield.rs` 5ef5edc7661c -> 3094082e80b8. Reason: init_greenfield.rs ships a revision log with a dated first row in every built-in template; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Pin recomputed on merge (attested by beauwilliams, identity: gh-verified, actor: agent): both merged sides re-pinned, `rust/crates/urzua-cli/src/commands/init_greenfield.rs` 15c7e3815148 and 3094082e80b8 -> b6fdfdbb3652; the new hash is the merged file's, which is git's automatic merge of the two re-pinned commits. No person wrote a reason for the merged state. **Why:** `urzua fix --conflicts` resolved a conflict that lay only in re-pin lines; a reviewer of the pull request should confirm the merged pin. | **structural** |
> | 2026-11-16 | Review of `MILE-265`'s first round: a numberless file with a header stays a record (only index pages, and numberless unheaded files in a headerless directory, are excluded); a headerless type needs a decision-record heading from a built-in vocabulary or a decision-record directory name, and a directory that fails it is named as ordinary pages with how to declare it; refusals give their real reason; heading spellings that differ in case or a trailing colon count as one; a partly headed directory gets inferred sections; `config.known-fields-declaration-missing` is withheld. Same `0.34`. Decided by the builder on 2026-11-16, pending maintainer review. **Why:** an independent review of PR 550 found a misnamed record excluded, tutorial pages adopted and a first `check` that warned about the proposed config. | **substantive** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` ded385eeb996 -> c8dc9d5fd2a8, `rust/crates/urzua-cli/tests/init_adopt_integration.rs` 566bb95965e7 -> e4f6e92a173b. Reason: init.rs: adopt proposes header_shape none for decision-shaped headerless directories with inferred sections, excludes only index pages, names pages with reasons, withholds rules that examine nothing; init_adopt_integration.rs: adds tests for headerless, mixed and partly headed corpora, exclusions, tutorial pages, refusal reasons, heading spellings and migrate ids **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` c8dc9d5fd2a8 -> 5fca64adc779. Reason: Read the diff of each file against origin/main: init.rs adds GREENFIELD_ARGV beside GREENFIELD_COMMAND and passes the not-run remedy as words. The cited behavior is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
