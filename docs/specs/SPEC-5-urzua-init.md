---
Stable-Id: 01M6R3NK5PKKQRJF9WFZJA105B
Version: '0.20'
Date: 2026-08-20
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:ee569c2dd2fff94a516fab4d09e931209aedfa6d0d4e570de132e42d2bda6ec0, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:5ab61b95fd5864a41566dee0d902fac000dca3552f90316331ecf90617c3f9b0
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

`--config <file>` names the file `init` writes (and the one it refuses to overwrite); without it the file is `.urzua/config.yaml`. A default location is checked to resolve inside the repository, so a symlinked `.urzua` pointing outside is refused rather than written through.

`init` also proposes a directory whose files are not numbered records when at least two of its markdown files carry a header and they are at least half of the directory's markdown files (a flat spec directory named by slug), with no `prefix`, and says so in an `init-scan` notice. A proposed directory none of whose files has a header is named in an `init-scan` warning notice, since it may hold pages rather than records; it is not proposed (a type with nothing to read is noise), and the notice says to declare it by hand if it is a record directory. A hidden directory (any path component starting with a dot, such as `.changeset`) is never proposed or mentioned, a leading dot being neither a legal type name nor a legal prefix. An unnumbered directory needs, beyond the header test above, at least two headed files carrying a `status` field (any letter case); a directory of headed pages without one (front matter holding only `title`) is named in a notice and not proposed. A type name is lower-case ASCII letters and digits in hyphen-separated segments with no all-digit segment, so its upper-cased default prefix always loads; a directory whose records share a prefix an earlier directory already holds is not proposed and is named in a notice. The rendered config is parsed before anything is written, and one that does not load is refused.

## Proposed header shape

`init` proposes `header_shape: yaml-frontmatter` for every type, the only shape with a reader (the
`blockquote` and `bold-list` shapes were removed, `MILE-85`). For each proposed type it parses the
existing records with the real header parser and counts those whose header is YAML frontmatter at the
very start of the record; a record written in a removed form, or with no readable header, is counted
in a `header-shape` notice that names the form (`2 in the removed bold-list form`), which no rule reads
until its header is rewritten as YAML frontmatter. The proposed config declares no `required_fields`, so
`check` does not mention such a record; the notice says so and names the fix, declaring a field every
record carries (`required_fields: [Status]`) on the type, after which `header.required-fields` reports
each one. A type whose records are all frontmatter gets no notice. A type with no records is proposed
`yaml-frontmatter`.
Each `proposed` entry reports the chosen `header_shape`. When a proposed directory holds a `README.md`
directly, which is never a record, the type is written with `exclude_files: [README.md]` and its `proposed`
entry lists `exclude_files`; a directory with no README writes no `exclude_files`.

## Proposed harness key

`init` also proposes `harness: {outside_writes: shared}`. It enables no guardrail; it makes a guardrail the
adopter turns on later also judge the files of a linked worktree of the repository (`ADR-89` amendment of
2026-11-09, outside writes; `SPEC-3`).

## Proposed rules

`init` proposes each shipped rule at `warn`, except a rule that cannot examine anything on the corpus it
just adopted: a rule that needs options it cannot supply, a rule that needs declared `relation_pairs`,
a rule that resolves identity on a corpus where no type has a prefix, and a rule that reads a per-type
declaration `init` does not write. `init` writes `required_fields: []` and infers no `known_fields` or
`pointer_fields` (`ADR-33`), so `field.quality`, `field.pending`, `header.field-set-consistency` and
`header.pointer-field-clean` are not proposed. A `rule-applicability` notice (severity `info`) lists them
with the declaration each reads, so the adopter knows what to declare on a record type before enabling
the rule. Each rule's declaration is the `reads_declaration` of its entry in the rule table.

`init` also writes `authority: {decider_table: base}`: no deciders and no gates, which no corpus can supply,
only the choice that who may decide is read from a pull request's base revision (`SPEC-3`, `ADR-89`
amendment of 2026-11-09), so the table the adopter writes next is not one a pull request can edit for itself.

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
> | 2026-11-07 | Bumped to `0.12`. `init` writes the file named by the global `--config`, and checks the default location resolves inside the repository. **Why:** a Phase 0 hardening review found `--config X init` reported `written: true` for a file it never created, writing `.urzua/config.yaml` instead. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: init.rs and check_integration.rs still implement and test urzua init adopt mode (SPEC-5), status_model.rs, the own-config test and status_delivery_integration.rs still implement the milestone type and its delivery status (SPEC-6), both fix.rs files and the paths, rebaseline and drift integration tests still implement urzua fix (SPEC-8), and config.yaml and the own-config tests still declare the bug type (SPEC-9). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082148a47756ba8d8a4b6e8dc7d85f66edb3748c63e7a7e5f01c96c, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b8337c1172d3c4825859e00a0344dd4c35e6f013d802e509b1688ecb` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-08 | Bumped to `0.13`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` efe3f3739082 -> 08dfadd3cae6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. init.rs: only the doc comment on run changed, to say the config a dry run proposes is byte-identical to the file the real run writes and that the reports differ in dry_run, written and the echoed config_yaml; no code changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.14`. The pins this spec's `Realized-by` holds on files changed in this change were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Bumped to `0.15`. The `header-shape` notice states what `check` does (silent while `required_fields` is empty) and the config that makes it report, instead of promising a report that never came; `init` writes `exclude_files: [README.md]` for a proposed directory that holds a README. The written config for a corpus with no README, and every existing config, are unchanged. **Why:** the round-4 adopter review (S3, N1): the warning was not true, and every adopted corpus drew `filename.identity-pattern` warnings asking to rename its README. | **substantive** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 08dfadd3cae6 -> 5f376bdd1b6d, `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged; init.rs now words the header-shape notice truthfully (check reports unreadable headers only when required_fields declares them), excludes README.md for a directory holding one, and carries the merged doc-comment edit from main; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 5f376bdd1b6d -> 6c671b5f6ed7. Reason: split init::run into propose_record_types and write_config, and render_config_yaml into proposed_types_mapping and proposed_rules_mapping (init.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 6c671b5f6ed7 -> eab52da49062. Reason: Read the diff of each re-pinned file against main. cli/commands/init.rs: Notice literals use Notice::new or gain detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Bumped to `0.18`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.19`. Adds `## Proposed harness key`: the proposed config declares `harness.outside_writes: shared`. **Why:** `ADR-89` amendment of 2026-11-09 (outside writes), accepted by the maintainer: a linked worktree at a commit without gates was an unjudged place to write. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` eab52da49062 -> 1520764ba016. Reason: init.rs: read the diff since the previous pin; render_config_yaml adds a top-level harness mapping {outside_writes: shared} to the proposed config (6 added lines, no other change to the file). This record's claim about init is unaffected; SPEC-5 0.19 documents the new key. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.20`. The proposed configuration carries `authority: {decider_table: base}`, with no deciders or gates. **Why:** `ADR-89` amendment of 2026-11-09 (decider table), accepted by the maintainer. | **substantive** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 1520764ba016 -> ee569c2dd2ff. Reason: Read the diff of each pinned file since the old pin and the claim still holds. init.rs: render_config_yaml adds an authority mapping holding only decider_table: base to the proposed configuration; nothing else in the pinned file changed. Behaviour unchanged for a repository that does not declare decider_table. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
