---
Stable-Id: 01M6R3NK5PKKQRJF9WFZJA105B
Version: '0.11'
Date: 2026-08-20
Status: Accepted
Embodiment: Drift detected
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs, test:rust/crates/urzua-cli/tests/check_integration.rs
Author: beauwilliams
Subject: '`urzua init` -- the adopt-existing-corpus path and `.urzua/` layout.'
Implements: RFC-1
Parent: SPEC-1
---
# SPEC-5 — `urzua init`

## Purpose

`init` is where a repository acquires a governed corpus, and it is the first sixty seconds of the
product. Today that means adopt: it infers which record types already exist from the tree and
produces the configuration every other command reads. Choosing record types for a repository that has
none yet is `MILE-114`'s scope.

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
  templates/            tracked — a hand-authored starting point per type, read by `new` (SPEC-12)
    adr.md                if present; init does not generate one
    rfc.md
  cache/                intended gitignored — derived, never authoritative; not yet used by any command
```

(`.gitignore` gaining `.urzua/cache/` is not yet built either -- `init` does not touch `.gitignore`
today.)

`init` writes only `.urzua/config.yaml` today. `templates/` and `cache/` are part of `.urzua/`'s
settled shape -- `new` already reads a template from `templates/<type>.md` when a repository hand-
authors one (`SPEC-12`) -- but nothing in the write path generates either, and `MILE-114`'s greenfield
mode is the natural place for `init` to start writing starter templates once it exists.

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

`init` takes only `--dry-run` and the global `--config`; adopt infers types from what's already on
disk rather than taking a `--types` selection. Type selection, built-in profiles, and non-interactive
setup for a repository with no records yet are `MILE-114`'s scope, not this spec's current subject.

`init` also proposes a directory whose files are not numbered records when at least two of its markdown files carry a header and they are at least half of the directory's markdown files (a flat spec directory named by slug), with no `prefix`, and says so in an `init-scan` notice. A proposed directory none of whose files has a header is named in an `init-scan` warning notice, since it may hold pages rather than records; it is not proposed (a type with nothing to read is noise), and the notice says to declare it by hand if it is a record directory. A hidden directory (any path component starting with a dot, such as `.changeset`) is never proposed or mentioned, a leading dot being neither a legal type name nor a legal prefix. An unnumbered directory needs, beyond the header test above, at least two headed files carrying a `status` field (any letter case); a directory of headed pages without one (front matter holding only `title`) is named in a notice and not proposed. A type name is lower-case ASCII letters and digits in hyphen-separated segments with no all-digit segment, so its upper-cased default prefix always loads; a directory whose records share a prefix an earlier directory already holds is not proposed and is named in a notice. The rendered config is parsed before anything is written, and one that does not load is refused.

## Proposed header shape

`init` proposes `header_shape: yaml-frontmatter` for every type, the only shape with a reader (the
`blockquote` and `bold-list` shapes were removed, `MILE-85`). For each proposed type it parses the
existing records with the real header parser and counts those whose header is YAML frontmatter at the
very start of the record; a record written in a removed form, or with no readable header, is counted
in a `header-shape` notice that names the form (`2 in the removed bold-list form`), because `check`
reports each such record as unreadable until its header is rewritten as YAML frontmatter. A type whose
records are all frontmatter gets no notice. A type with no records is proposed `yaml-frontmatter`.
Each `proposed` entry reports the chosen `header_shape`.

## Proposed rules

`init` proposes each shipped rule at `warn`, except a rule that cannot examine anything on the corpus it
just adopted: a rule that needs options it cannot supply, a rule that needs declared `relation_pairs`,
a rule that resolves identity on a corpus where no type has a prefix, and a rule that reads a per-type
declaration `init` does not write. `init` writes `required_fields: []` and infers no `known_fields` or
`pointer_fields` (`ADR-33`), so `field.quality`, `field.pending`, `header.field-set-consistency` and
`header.pointer-field-clean` are not proposed. A `rule-applicability` notice (severity `info`) lists them
with the declaration each reads, so the adopter knows what to declare on a record type before enabling
the rule. Each rule's declaration is the `reads_declaration` of its entry in the rule table.

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

Reporting a record-shaped file adopt could not classify, by path, rather than silently excluding it
is `MILE-114`'s scope: today adopt reports only what it did classify.

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

`config_yaml` (the full rendered config) is present only when `dry_run: true` -- there's nothing
else to read the preview from, since nothing was written. A real write omits it, matching `new`'s own
established convention (SPEC-12): the caller reads the file at `config_path` if it needs the content.

## Exit codes (shipped)

| Code | When |
|---|---|
| 0 | proposal computed (dry-run) or config written |
| 2 | refused via `CouldNotRun` -- existing config, no record-shaped files found, unwritable path |

Exit `1` ("adopt completed with unclassified files") is `MILE-114`'s scope, not built -- adopt today
classifies every record-shaped file it finds by directory; nothing is reported as unclassified.

## Success criteria

1. `--dry-run` output matches what a real run then does, byte for byte.
2. Adopt moves no files, and a `git status` after it shows only `.urzua/` and `.gitignore`.

Criterion 2 is the one to hold. The moment `init` moves a file it becomes a migration, and a
migration without a reverse-reference scan is the documented data-loss shape.

A `urzua init --types adr,rfc,spec` run producing a checkable config with unclassified files reported
rather than errored, and a no-op re-run exiting 0 rather than 2, are `MILE-114`'s success criteria, not
this spec's current subject.

## Open questions

None currently open for the adopt path this spec describes. `MILE-114` carries the open questions
that depend on greenfield mode existing first (CI/hooks wiring, the display-number scheme, whether
adopt should infer rules or only structure, and multi-corpus-per-repo).

## References

- SPEC-2 — `check`, the consumer of what this writes.
- SPEC-3 — configuration; amended by this spec to live at `.urzua/config.yaml`.
- RFC-1 — core+profile, which `MILE-114`'s type selection will instantiate.
- RFC-11 — why an ignore list is the wrong answer to the template problem.
- MILE-114 — greenfield mode, `--types`/`--dir`, built-in profiles, and unclassified-file reporting,
  relocated from this spec's own Draft sections (`BUG-26`).

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
