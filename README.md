# Urzua

[![ci](https://github.com/urzuahq/urzua/actions/workflows/ci.yml/badge.svg)](https://github.com/urzuahq/urzua/actions/workflows/ci.yml)
[![release](https://github.com/urzuahq/urzua/actions/workflows/release.yml/badge.svg)](https://github.com/urzuahq/urzua/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**Your agent just wrote a record of a decision it made. Is it actually structured data, or is it prose that happens to have a `Status:` line at the top?**

Agents and humans both write governance records now — ADRs, RFCs, specs, decision logs, anything with a lifecycle and a relationship to other records. Almost all of that ends up as free-form markdown with a header someone remembered to fill in. A field goes blank and nobody notices. A cross-reference points at a record that was deleted. A decision says `Implemented` and the code that was supposed to implement it doesn't exist. Nothing catches any of it, because nothing treats the record as **data** — just as a document a human might one day reread.

**Urzua is a record engine, not a record format.** You declare what a record type looks like — its fields, its header shape, its required relationships — and Urzua validates, tracks, and repairs against that declaration. It doesn't assume you're writing ADRs specifically, and it doesn't force a rewrite of records you already have.

## What a record looks like

Two valid shapes for the same fact, because Urzua reads whatever a corpus already uses and writes something more structured going forward.

**An existing corpus, unchanged** — this is real, working `blockquote` shape:

```markdown
# 0016 — A bold-list header shape, declared per profile

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs
> Derives-from: RFC-0010 (Accepted)

## Context
...
```

**A record Urzua generates going forward** — genuinely structured YAML frontmatter, not text lines a regex pulls apart:

```markdown
---
Status: Accepted
Embodiment: Verified
Realized-by: "code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs"
---
# 0016 — A bold-list header shape, declared per profile

## Context
...
```

Same schema underneath — `check`, `fix`, and every rule read either shape identically. The difference is only how the header is stored: line-recognized for a corpus that already exists, or `serde`-deserialized for one Urzua writes itself. See [`docs/rfc/0016-yaml-frontmatter-as-the-generated-record-header.md`](docs/rfc/0016-yaml-frontmatter-as-the-generated-record-header.md) for why both need to exist. `Realized-by`'s own value is still a flat `type:locator` string under either shape today — [`docs/rfc/0005-embodiment-realized-by-claim-graph.md`](docs/rfc/0005-embodiment-realized-by-claim-graph.md) designs the fuller graph-shaped version this release deliberately ships a narrower first slice of.

## Why now

Governance records used to be slow to produce and easy to keep consistent by hand — one architect, one meeting, one document. Neither is true anymore:

- **Records are machine-authored at machine speed**, and nothing validates them as data until a human notices something's wrong, usually much later.
- **A record's relationships rot silently.** `Implements: RFC-0001` still parses as valid text long after RFC-0001 is deleted, renamed, or superseded.
- **"Was this actually built" and "was this decided" get conflated.** A record can say `Accepted` and `Implemented` in the same header with nothing distinguishing a real decision from a stale claim about the code.
- **Every team reinvents the same validator**, badly, once per repo, because there's no portable engine — just a format convention and whatever regex someone wrote against it last time.

Urzua is built to be the thing every hand-written record linter is quietly reinventing: one engine, a declared schema instead of an assumed one, and evidence-backed claims instead of prose someone promises is still true.

## What Urzua actually checks

Real rules, running against this repository's own `docs/` in CI on every commit — see [status](#status) for what's implemented versus designed but not yet built.

| Rule | Catches |
|---|---|
| `header.required-fields` | A required field missing, or the header itself not found at all |
| `pointer.resolution` | `Implements:`/`Derives-from:` pointing at a record that doesn't exist |
| `field.quality` | Blank vs. unedited template text vs. an explicit pending marker vs. a real value — three distinct failure states one presence check can't tell apart |
| `filename.title-consistency` | A renamed file whose title didn't get renamed with it (or vice versa) |
| `relation.supersession-reciprocity` | A claims to supersede B, but B doesn't point back |
| `revision-log.change-class-required` | A revision-log entry with no real `substantive`/`structural` classification |
| `embodiment.consistency` | A record's stated `Embodiment` disagreeing with what its own cited evidence computes to |
| `embodiment.locator-promotion-candidate` | The same piece of evidence cited by more than one record, drifting independently instead of being tracked once |

That last pair is the part most linters don't have at all: a record can name what actually realizes it — a spec, a source file, a test — categorized by strength of evidence, and `check` computes whether the record's own claim still matches. Disagree, and it's a finding, not a stale comment nobody re-reads.

**Stdout is the JSON report, unconditionally (ADR-0023/0026) — no `--format` flag, no second human-readable rendering anywhere.** There is one shape. An agent piping stdout, a script, and a person reading a terminal all see exactly the same thing:

```
$ urzua check docs/
{
  "status": "findings-present",
  "files_examined": 44,
  "rules_executed": [
    { "rule": "header.required-fields", "records_examined": 44 },
    { "rule": "embodiment.consistency", "records_examined": 6 }
  ],
  "findings": [
    {
      "rule": "embodiment.locator-promotion-candidate",
      "severity": "warning",
      "file": "docs/adr/0016-...md",
      "message": "locator 'rust/crates/urzua-core/src/header.rs' is cited by 2 records -- consider promoting to a shared claim record"
    }
  ]
}
```

## Repair, not just detect

`urzua fix` closes the loop `check` opens: where a field's correct value is mechanically derivable — not authored, not judged, just computed — Urzua can write it back, gated hard.

```
$ urzua fix
{
  "status": "repairs-available",
  "records_examined": 6,
  "repairs": [
    {
      "record": "docs/adr/0042-example.md",
      "field": "Embodiment",
      "current_value": "Not started",
      "computed_value": "Verified",
      "tier": 1,
      "evidence": "Realized-by: code:src/lib.rs, test:tests/it.rs"
    }
  ]
}

$ urzua fix --apply --ids docs/adr/0042-example.md --by beau
```

Every applied write: touches only the one field's line, byte-for-byte preserving the rest of the record; requires a resolved identity (`--by`, or `gh api user`, or `git config user.name`); appends a structural entry to the record's own revision log — and refuses outright, per-record, if that log doesn't exist rather than writing somewhere it can't be audited. Rationale, alternatives, and every hand-authored sentence stay permanently off-limits — this is a cache of a computation, never an editor.

## Adopt without a rewrite

```sh
git clone https://github.com/urzuahq/urzua && cd urzua && make build
cd /path/to/your-repo
/path/to/urzua/rust/target/release/urzua init --dry-run # preview .urzua/config.toml from what's already there
/path/to/urzua/rust/target/release/urzua init           # writes it; never moves or rewrites a single record
/path/to/urzua/rust/target/release/urzua check docs/    # validates against it
```

`init` reads your existing directories and header conventions and proposes a config — it never renumbers, never rewrites, never moves a file. Nothing about your corpus changes until you accept what it found. Confirm this yourself rather than trust it: `make ci` in this repo runs the same `urzua check docs/` against Urzua's own corpus, and [`ci.yml`](.github/workflows/ci.yml) runs it on every push and pull request.

## The schema is declared, not assumed

A record type is whatever `.urzua/config.toml` says it is — Urzua ships no hardcoded notion of "an ADR" or "an RFC."

```toml
[record_types.adr]
dir = "docs/adr"
required_fields = ["Status", "Date", "Author", "Deciders"]
header_shape = "blockquote"       # or "bold-list" / "yaml-frontmatter"

[record_types.incident-review]     # any record type your org actually uses
dir = "docs/postmortems"
required_fields = ["Status", "Severity", "Owner"]
header_shape = "yaml-frontmatter"
```

Every existing header convention this project has actually encountered is a declared shape, not a guess: a blockquote (`> Key: Value`), a bold-labelled blockquote, a pipe-delimited single line, a bold markdown list with no blockquote at all, and YAML frontmatter. A misdeclared shape fails loud — `no header-shaped region found` — never a silent match on the wrong lines.

## Status

**v0.1.0, self-hosting.** Real and validated against this repository's own `docs/` on every commit:

| Command | State |
|---|---|
| `check` | Real — full rule set above, always JSON on stdout (ADR-0023) |
| `explain <path>` | Real — every record whose `Realized-by` names this file as evidence (ADR-0024) |
| `graph` | Real — the full `Implements`/`Derives-from`/`Supersedes` relationship graph, as data, dangling edges flagged (ADR-0024) |
| `init` | Adopt mode only — proposes config from an existing corpus, never moves files |
| `fix` | Detect mode (Embodiment, Tier 1) and apply mode, both real |
| `migrate ids` | Real — backfills a collision-free stable ID ([ULID](docs/adr/0021-ulid-as-the-stable-id-encoding.md)) into every record lacking one |
| `migrate schema --report` | Real — previews which records lack a real value for a proposed new required field, before it's ever added to config |
| `doctor` | Real — reports on the tool's own configuration health, not record content |
| `new`, `audit`, `migrate schema --assist-waivers`/`--apply`, `export`, `import` | Not implemented yet — exit 2 with "not implemented yet" |

See [`CHANGELOG.md`](CHANGELOG.md) for what shipped when, and [`docs/rfc/`](docs/rfc/) / [`docs/adr/`](docs/adr/) for what's designed but not yet built — including the fuller claim-graph model (AND/OR composites, cross-record-shared claims) this release's Embodiment tracking deliberately ships a narrower slice of first.

## Maintenance

Released for use, actively maintained — this is a real tool being built in the open, not published only for reference. Issues and PRs are triaged. The schema and CLI contract may still change before `v1.0.0`; `CHANGELOG.md` calls out anything breaking.

## Layout

Polyglot monorepo, organized by language at the root ([ADR-0004](docs/adr/0004-polyglot-monorepo-layout.md)):

```
rust/           Cargo workspace — urzua-core (pure schema+validation), urzua-io
                (git/filesystem, identity resolution), urzua-id (stable
                identifiers), urzua-agdr (AgDR export format), urzua-cli
                (the `urzua` binary)
ts/             Reserved for agent-harness integrations. Empty until there's real content.
platform/       Reserved for deployment targets. Empty until there's real content.
docs/adr/       Decisions made about Urzua itself.
docs/rfc/       Pre-decision proposals — the design record, read these first.
docs/specs/     Build-level detail for what ships.
```

Build with `make`, not `cargo` directly — `make build` / `make test` / `make ci` work without
knowing which languages are involved, though native commands work inside `rust/` too. `make help`
lists every target.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Telemetry

Urzua collects nothing and phones home never. It reads the files you point it at and writes to
stdout/stderr and the paths you configure. No network calls, no analytics, no crash reporting.

## License

[MIT](LICENSE).
