# Contributing

Released for use, actively maintained — issues and PRs are triaged. The record schema and CLI
output contract are largely decided (see `docs/adr/` for what's `Accepted`), but a handful of open
questions remain in `docs/rfc/` (RFC-0011 §6's scope-as-glob-vs-subject question, in particular) —
changing the schema after real adopters exist is a migration, not a docs edit, so treat those as
genuinely open rather than settled.

## Clone to productive

```sh
git clone https://github.com/urzuahq/urzua && cd urzua
make ci        # exactly what CI runs: fmt-check, clippy, build, test
make hooks-install   # optional: fmt-check + clippy on every push
```

`make help` lists every target. Native `cargo` commands work fine inside `rust/` for anyone who
prefers them; `make ci` is the thing CI actually runs, so it's the one to trust before pushing.

## Adding a rule to `urzua check`

Every rule lives in `rust/crates/urzua-core/src/rules.rs`, returns a `(RuleExecution, Vec<Finding>)`
pair, and reports the number of records it actually examined — not just whether it ran. **Every rule
ships with a planted-violation test**: construct the exact defect the rule exists to catch, and
assert it's caught. A rule without one is unverified, not trusted, per RFC-0011 §5.

## What's useful right now

- **Reading the RFCs and filing an issue with a specific objection.** "RFC-0011 §6 — here's a case
  where the glob form breaks against a real rename" is far more useful than a general comment.
- **Running `urzua check`/`urzua init` against your own decision-record corpus** and filing what
  breaks — see [`docs/specs/0004-the-corpus-acceptance-suite.md`](docs/specs/0004-the-corpus-acceptance-suite.md)
  for the documented bug classes this project already tests against.

## Ground rules

- **Every RFC follows [`docs/rfc/_template.md`](docs/rfc/_template.md) exactly** — banner block,
  then Summary / Motivation / Proposal / Open questions / Non-goals / References.
- **Open questions stay open.** Don't smooth over uncertainty to make a proposal look more decided
  than it is.
- **A `Status` value is never silently rewritten** during an otherwise structural edit. Substantive
  changes get called out as substantive.
- **A waiver, not an ignore list.** If `urzua check` flags something in your own corpus that's a
  reviewed exception, see ADR-0011 for the waiver record shape — don't reach for a config-level
  exclusion.
