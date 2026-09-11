---
Version: '0.4'
Date: 2026-08-20
Status: Draft
Author: '@beauwilliams'
Subject: '`urzua init` -- record-type selection, `.urzua/` layout, and the adopt-existing-corpus path.'
Implements: RFC-1
Parent: SPEC-1
---
# SPEC-5 — `urzua init`

## Purpose

`init` is where a repository acquires a governed corpus, and it is the first sixty seconds of the
product. It selects which record types a repository keeps, writes the layout for them, and produces
the configuration every other command reads.

It is also where the multi-document-type position becomes visible. Nearly every tool in the
landscape is ADR-only; RFC-1's core+profile model spans types by design, and `init` is the
surface where a user sees that they may have ADRs, RFCs, specs and PRDs under one schema. `init`
instantiates profiles — it does not invent a second concept.

## The two modes, and which one matters

| Mode | Trigger | What it does |
|---|---|---|
| **Adopt** | record-shaped files already exist | proposes a config describing what is already there; moves nothing |
| **Greenfield** | no records found | creates directories and templates for the selected types |

**Adopt is the primary case.** Every codebase this project was extracted from already had records
before it had tooling, and a tool whose setup path assumes an empty repository asks its most likely
user to migrate before they can evaluate it. Greenfield is the easy case and is not the interesting
one.

Adopt **never moves a file.** It reads the tree, proposes a config, and reports what it found and
what it could not classify. Restructuring is `migrate`'s job and carries a reverse-reference scan;
folding that into `init` would put a corpus-wide move behind a command whose name promises setup.

## Layout

Everything the tool owns lives under `.urzua/`:

```
.urzua/
  config.toml           tracked — the one config
  templates/            tracked — one per selected type
    adr.md
    rfc.md
  cache/                gitignored — derived, never authoritative
.gitignore              gains .urzua/cache/
```

**Why a directory rather than a root `urzua.toml`.** Config alone would not need one; config plus
templates plus derived state does, and scattering those three across the root, the corpus, and a
cache directory is how each ends up governed by a different rule.

**Why templates leave the corpus.** This is the load-bearing reason, and it comes from this
repository. `docs/adr/_template.md` and `docs/rfc/_template.md` are git-tracked, sit inside the
record directories, and contain `Status: Proposed | Accepted | Rejected | Superseded`,
`Date: YYYY-MM-DD` and `# NNNN — Title`. Every one of those is invalid as a *record* and correct as
a *template*. A checker that discovers them reports errors on files that are exactly right.

The reflex is an ignore list. An ignore list is an ad-hoc exclusion that grows without bound, is
invisible to the reader who has the question, and is the same anti-pattern RFC-11 rejects for
boundaries and RFC-13 rejects for numbering gaps. Moving templates out of the corpus removes the
question instead of suppressing it: the template is not a record, so it does not live where records
live, and no rule needs to know it exists.

`cache/` is gitignored because derived state that is committed becomes a thing to reconcile. Nothing
in `cache/` may be required for a correct run — a fresh clone with no cache produces identical
results, only slower.

## Type selection

```
urzua init                          # interactive: pick types
urzua init --types adr,rfc,spec     # non-interactive
urzua init --types adr --dir docs   # layout root
```

Built-in profiles ship for `adr`, `rfc`, `spec` and `prd`. Each supplies a directory, a template,
a status enum, required fields per status, and role requirements — the per-type half of RFC-1's
core+profile model, with the core fixed across all of them.

A profile is a **starting point that is written out, not a hidden default.** `init` materializes the
profile's rules into `config.toml` rather than referencing a built-in by name. A user who disagrees
with a required field edits a visible line; a built-in referenced by name is a rule nobody can see
and nobody reviewed, which is the failure this project exists to remove.

Non-interactive must be first-class, not an afterthought: `init` runs in CI, in containers, and
under agents, and an interactive-only setup path is unusable by exactly the callers this product is
for.

## Safety

- **Never clobber.** An existing `.urzua/config.toml` is not overwritten. `init` reports what
  exists and exits 2.
- **Idempotent.** Re-running against an initialized repository is a no-op that reports the current
  state — the same property that makes it safe in a provisioning script.
- **`--dry-run` prints the plan and writes nothing**, and is the documented way to see what adopt
  inferred before committing to it.
- **Adopt reports what it could not classify**, by path. A file under a record directory that does
  not parse is surfaced, never silently excluded — the whole point of the command is to say what it
  found.

## Output (shipped)

Adopt mode only -- greenfield mode, `--types`/`--dir`, and `unclassified`-file tracking below this
heading are still the Draft, not-yet-built design; the actual shipped shape (`Report`/ADR-46) is
narrower:

```json
{
  "status": "ok",
  "dry_run": false,
  "config_path": ".urzua/config.toml",
  "proposed": [{ "name": "adr", "dir": "docs/adr", "record_count": 5 }],
  "written": true
}
```

`config_toml` (the full rendered config) is present only when `dry_run: true` -- there's nothing
else to read the preview from, since nothing was written. A real write omits it, matching `new`'s own
established convention (SPEC-12): the caller reads the file at `config_path` if it needs the content.

## Exit codes (shipped)

| Code | When |
|---|---|
| 0 | proposal computed (dry-run) or config written |
| 2 | refused via `CouldNotRun` -- existing config, no record-shaped files found, unwritable path |

Exit `1` ("adopt completed with unclassified files") is part of the Draft design above, not built --
adopt today classifies every record-shaped file it finds by directory; nothing is reported as
unclassified.

## Success criteria

1. `urzua init --types adr,rfc,spec` on this repository produces a config under which
   `urzua check` runs, with both templates reported as unclassified rather than as errors.
2. Re-running changes nothing and exits 0.
3. `--dry-run` output matches what a real run then does, byte for byte.
4. Adopt moves no files, and a `git status` after it shows only `.urzua/` and `.gitignore`.

Criterion 4 is the one to hold. The moment `init` moves a file it becomes a migration, and a
migration without a reverse-reference scan is the documented data-loss shape.

## Open questions

- **Does `init` wire CI and hooks?** Enormous convenience, and it writes to files it does not own
  (`.github/workflows/`, `.husky/`). Leaning opt-in behind `--with-ci` and printing the snippet
  otherwise — a tool that silently edits a workflow is a tool nobody trusts in a shared repo.
- **Where does the display-number scheme get chosen?** Zero-padded width, per-type or global
  sequence, and whether stable IDs are visible in filenames are all init-time choices that are
  expensive to change later, and ADR-3 has not settled the encoding.
- **Should adopt infer rules, or only structure?** Inferring that every existing ADR has a
  `Deciders` field and therefore requiring it is powerful and quietly encodes today's corpus as
  policy, including its accidents.
- **Is `.urzua/` right when a repo holds several unrelated corpora?** One config per repo is
  SPEC-3's position; a monorepo with genuinely independent products is the case that tests it,
  and scope (RFC-11 §6) may already be the answer.

## References

- SPEC-2 — `check`, the consumer of what this writes.
- SPEC-3 — configuration; amended by this spec to live at `.urzua/config.toml`.
- RFC-1 — core+profile, which type selection instantiates.
- RFC-11 — why an ignore list is the wrong answer to the template problem.
- ADR-3 — identifiers; the unsettled encoding behind open question 2.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Initial spec. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-09-11 | Added `## Output (shipped)`/`## Exit codes (shipped)` documenting the real, narrower `InitReport` shape (BUG-20, ADR-46), separated from the Draft's own richer aspirational design (greenfield mode, `--types`/`--dir`, `unclassified` tracking, exit 1) so a reader can tell which parts are built. | **substantive** |
