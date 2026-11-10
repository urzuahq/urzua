# Contributing

Released for use, actively maintained — issues and PRs are triaged. The record schema and CLI
output contract are largely decided (see `docs/adr/` for what's `Accepted`), but a handful of open
questions remain in `docs/rfc/` (RFC-11 §6's scope-as-glob-vs-subject question, in particular) —
changing the schema after real adopters exist is a migration, not a docs edit, so treat those as
genuinely open rather than settled.

## Clone to productive

```sh
git clone https://github.com/urzuahq/urzua && cd urzua
make build     # builds the binary and wires the git hooks (core.hooksPath=.git-hooks)
make ci        # exactly what CI runs: fmt-check, clippy, build, test
```

The first `make build` of a clone wires the versioned hooks: `pre-commit` runs `urzua check --base HEAD`
and `pre-push` runs fmt, clippy and `urzua check`. `make hooks-install` does only the wiring. When
the binary is not built, or `check` cannot run, the commit hook prints a `NOT RUN` banner on stderr
and lets the commit through, because CI is the guarantee (ADR-71); blocking findings stop the
commit. In Claude Code an editor hook (`.claude/settings.json`, ADR-69) also judges a `Write` or
`Edit` of a governed record before it is written and blocks an error the edit introduced. `urzua doctor` reports each hook's state. If you already use a `core.hooksPath` of your own,
the build leaves it alone and says so.

`make help` lists every target. Native `cargo` commands work fine inside `rust/` for anyone who
prefers them; `make ci` is the thing CI actually runs, so it's the one to trust before pushing.

CI also fails a pull request when `urzua fix --rebaseline` lists a stale evidence pin; re-pin it with a per-file reason (`.claude/commands/repin.md`).

## Adding a rule to `urzua check`

Every rule lives in `rust/crates/urzua-core/src/rules.rs`, returns a `(RuleExecution, Vec<Finding>)`
pair, and reports the number of records it actually examined — not just whether it ran. **Every rule
ships with a planted-violation test**: construct the exact defect the rule exists to catch, and
assert it's caught. A rule without one is unverified, not trusted, per RFC-11 §5.

## Running the corpus acceptance suite

`cargo test -p urzua-cli --test acceptance` (also part of `make ci`) runs the cases under
`rust/crates/urzua-cli/tests/acceptance/corpora/`; a new corpus needs only a directory (records,
`manifest.yaml`, `expectations/*.yaml`), no code. A case marked `known_gap` documents a defect not yet
fixed and turns red once it is, as a reminder to remove the marker. To also run corpora kept outside
this repository, set `URZUA_ACCEPTANCE_EXTRA` to a `:`-separated list of directories laid out the same
way; unset, the runner prints that it skipped them. See SPEC-4.

## What's useful right now

- **Reading the RFCs and filing an issue with a specific objection.** "RFC-11 §6 — here's a case
  where the glob form breaks against a real rename" is far more useful than a general comment.
- **Running `urzua check`/`urzua init` against your own decision-record corpus** and filing what
  breaks — see [`docs/specs/SPEC-4-the-corpus-acceptance-suite.md`](docs/specs/SPEC-4-the-corpus-acceptance-suite.md)
  for the documented bug classes this project already tests against.

## Documenting a change

If your PR is something a person installing `urzua` would care about — a new command, a behavior
change, a bug fix — add a changeset: a markdown fragment in `.changeset/` describing it, written now
while the change is fresh rather than reconstructed later at release time (ADR-29).

```markdown
---
default: minor
closes: []
---

# Add `urzua new`

Creates a record from the configured template with a fresh stable ID assigned. Never asks you to
pick a number.
```

`default` is the package identifier (there's one package in this repo); the value is `major`,
`minor`, or `patch`. `closes` lists, on one line, the records the change closes (`closes: [BUG-60,
BUG-61]`, one id, or `closes: []` when it closes none); every fragment carries it, and `urzua check`
fails a fragment that lists a record still open, an id that names no record, or a value that is not
a one-line list (a block list is rejected by the release tool as well). It is a claim made as data:
a record the change merely mentions is not listed, and a mention in the text is never read as a
claim. The release tool ignores the key, so the records a release closes do not reach `CHANGELOG.md`;
a release note that wants them says so in the text (ADR-85). Everything after the closing `---` is the changelog entry itself — written for
the person installing the tool, not a restatement of the commit. Create one with `knope
document-change` (interactive) or write the file by hand, named anything ending in `.md`. CI fails a
PR with neither a changeset nor the `no-changeset` label (for changes with nothing to tell an
installer — CI config, internal refactors, docs fixes).

A maintainer cuts a release by running the `prepare release` GitHub Action manually: it consumes
every fragment, bumps the version, compiles `CHANGELOG.md`, tags, and pushes — the existing
tag-triggered release workflow takes it from there.

## Ground rules

- **Start every record with `urzua new <type> "<title>"`** (or `/rfc`, `/adr`, `/milestone` in
  Claude Code). It writes the YAML-frontmatter header and the section skeleton from
  `.urzua/templates/`; an RFC keeps Summary / Motivation / Proposal / Open questions / Non-goals /
  References.
- **Open questions stay open.** Don't smooth over uncertainty to make a proposal look more decided
  than it is.
- **A `Status` value is never silently rewritten** during an otherwise structural edit. Substantive
  changes get called out as substantive.
- **A waiver, not an ignore list.** If `urzua check` flags something in your own corpus that's a
  reviewed exception, see SPEC-10 for the waiver record shape — don't reach for a config-level
  exclusion.
