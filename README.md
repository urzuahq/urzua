# Urzua

[![ci](https://github.com/urzuahq/urzua/actions/workflows/ci.yml/badge.svg)](https://github.com/urzuahq/urzua/actions/workflows/ci.yml)
[![release](https://github.com/urzuahq/urzua/actions/workflows/release.yml/badge.svg)](https://github.com/urzuahq/urzua/actions/workflows/release.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![CodeRabbit Pull Request Reviews](https://img.shields.io/coderabbit/prs/github/urzuahq/urzua?utm_source=oss&utm_medium=github&utm_campaign=urzuahq%2Furzua&labelColor=171717&color=FF570A&link=https%3A%2F%2Fcoderabbit.ai&label=CodeRabbit+Reviews)](https://coderabbit.ai)

**Your agent just wrote a record of a decision it made. Is it structured data, or prose that happens to have a `Status:` line at the top?**

Urzua is a machine-native engine for structured records: decision logs, specs, incident reviews, a project's own backlog, and any record type with a lifecycle and relationships to other records. Every command returns a `status` and a list of typed `findings`: `"status": "findings-present"` is a value an agent branches on, `findings[]` is a list it loops over and filters by `severity`. Nothing needs interpreting first, and there's no guessing whether "3 issues" means stop or continue.

That same contract is what lets other tooling compose on top of it: a harness that chains `check` into `fix` into a PR comment, a CI gate that reads `status` and nothing else, an agent that treats Urzua as one step inside a longer plan. None of them re-interpret what happened; they read the same fields Urzua already computed.

It's a record engine, not a record format: you declare what a record type looks like (its fields, its header shape, its required relationships), and Urzua validates, tracks, and repairs against that declaration. It doesn't assume you're writing ADRs, and it isn't limited to decisions. This repo proves it on itself: `milestone` and `bug` were added after the fact, as ordinary config, with zero changes to the engine. The same tool that validates this project's decisions also tracks its own backlog and defects.

## What a record looks like

Two valid shapes for the same fact, because Urzua reads whatever a corpus already uses and writes something more structured going forward.

**An existing corpus, unchanged.** Real `blockquote` shape from this repo:

```markdown
# 16 — A bold-list header shape, declared per profile

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs
> Derives-from: RFC-10 (Accepted)

## Context
...
```

**A record Urzua generates going forward.** Real YAML frontmatter, not text lines a regex pulls apart:

```markdown
---
Status: Accepted
Embodiment: Verified
Realized-by: "code:rust/crates/urzua-core/src/header.rs, test:rust/crates/urzua-core/src/header.rs"
---
# 16 — A bold-list header shape, declared per profile

## Context
...
```

Same schema underneath: `check`, `fix`, and every rule read either shape identically. The difference is only how the header is stored: line-recognized for a corpus that already exists, or `serde`-deserialized for one Urzua writes itself. See [`docs/rfc/RFC-16-yaml-frontmatter-as-the-generated-record-header.md`](docs/rfc/RFC-16-yaml-frontmatter-as-the-generated-record-header.md) for why both need to exist. `Realized-by`'s own value is still a flat `type:locator` string under either shape today. [`docs/rfc/RFC-5-embodiment-realized-by-claim-graph.md`](docs/rfc/RFC-5-embodiment-realized-by-claim-graph.md) designs the fuller graph-shaped version this release deliberately ships a narrower first slice of.

## Why now

Governance records used to be written and read exclusively by humans: one architect, one meeting, one document a person kept consistent by hand. The writer is now at least as often an agent, and the reader needs to be a program too:

- **Records are machine-authored at machine speed**, and nothing validates them as data until a human notices something's wrong, usually much later.
- **A record's relationships rot silently.** `Implements: RFC-1` still parses as valid text long after RFC-1 is deleted, renamed, or superseded.
- **"Was this actually built" and "was this decided" get conflated.** A record can say `Accepted` and `Implemented` in the same header with nothing distinguishing a real decision from a stale claim about the code.
- **Every team reinvents the same validator**, badly, once per repo, because there's no portable engine, just a format convention and whatever regex someone wrote against it last time.

The evidence backs it up:

- Refactored code fell from 25% of changed lines (2021) to under 10% (2024) across 211M lines analyzed. Output is up, discipline is down. ([GitClear, 2025](https://www.gitclear.com/ai_assistant_code_quality_2025_research))
- 46% of developers don't trust AI output's accuracy, up from 31% a year earlier; 45% call debugging AI-generated code their top frustration. ([Stack Overflow, 2025](https://stackoverflow.co/company/press/archive/stack-overflow-2025-developer-survey/))
- "The same decision gets restated in `CLAUDE.md`, `AGENTS.md`, a README, and a wiki page, and the four copies disagree within a few months." ([Kennedy, 2026](https://www.actual.ai/blog/agent-optimized-adrs))
- Nothing "keeps the decision log connected," so teams accumulate "outdated ADRs, duplicated decisions, and uncertainty about the current architecture." ([Khan, 2025, LinkedIn](https://www.linkedin.com/pulse/agentic-ai-living-architecture-enhancing-adrs-llms-decision-khan-5gpgc))
- A spec "only stops drifting when it is connected to the work instead of sitting beside it." ([Ong, 2026](https://canery.ai/articles/why-specs-drift))

Even Google's own [AIP process](https://google.aip.dev/1) (200+ API decisions through a real review workflow) is a hand-maintained convention, not a mechanically checked one. Urzua replaces the reinvented validator with one engine, a declared schema, and evidence-backed claims instead of prose someone promises is still true.

## What Urzua actually checks

Real rules, running against this repository's own `docs/` in CI on every commit. See [status](#status) for what's implemented versus designed but not yet built.

| Rule | Catches |
|---|---|
| `header.required-fields` | A required field missing, or the header itself not found at all |
| `pointer.resolution` | `Implements:`/`Derives-from:` pointing at a record that doesn't exist |
| `field.quality` | Blank vs. unedited template text vs. an explicit pending marker vs. a real value: three distinct failure states one presence check can't tell apart |
| `filename.title-consistency` | A renamed file whose title didn't get renamed with it (or vice versa) |
| `relation.supersession-reciprocity` | A claims to supersede B, but B doesn't point back |
| `revision-log.change-class-required` | A revision-log entry with no real `substantive`/`structural` classification |
| `embodiment.consistency` | A record's stated `Embodiment` disagreeing with what its own cited evidence computes to, including drift: a locator that changed, per git history, since the `Realized-by` line was last touched ([ADR-32](docs/adr/ADR-32-drift-detection-via-git-blame-not-a-stored-hash.md)) |
| `embodiment.locator-promotion-candidate` | The same piece of evidence cited by more than one record, drifting independently instead of being tracked once |

That last pair is the part most linters don't have at all: a record can name what realizes it (a spec, a source file, a test), categorized by strength of evidence, and `check` computes whether the record's own claim still matches. Disagree, and it's a finding, not a stale comment nobody re-reads.

Every command's stdout is one JSON object, always, on every invocation (ADR-23/26). An agent branches on `status`, loops over `findings`, and acts on each one by `severity`, with nothing to translate first:

```
$ urzua check docs/
{
  "status": "findings-present",
  "files_examined": 98,
  "rules_executed": [
    { "rule": "header.required-fields", "records_examined": 98 },
    { "rule": "embodiment.consistency", "records_examined": 17 }
  ],
  "findings": [
    {
      "rule": "embodiment.locator-promotion-candidate",
      "severity": "warning",
      "file": "docs/adr/ADR-16-...md",
      "message": "locator 'rust/crates/urzua-core/src/header.rs' is cited by 2 records -- consider promoting to a shared claim record"
    }
  ]
}
```

## Repair, not just detect

`urzua fix` closes the loop `check` opens: where a field's correct value is mechanically derivable (not authored, not judged, just computed), Urzua can write it back, gated hard.

```
$ urzua fix
{
  "status": "repairs-available",
  "records_examined": 6,
  "repairs": [
    {
      "record": "docs/adr/ADR-42-example.md",
      "field": "Embodiment",
      "current_value": "Not started",
      "computed_value": "Verified",
      "tier": 1,
      "evidence": "Realized-by: code:src/lib.rs, test:tests/it.rs"
    }
  ]
}

$ urzua fix --apply --ids docs/adr/ADR-42-example.md --by beau
```

Every applied write touches only the one field's line, byte-for-byte preserving the rest of the record; requires a resolved identity (`--by`, or `gh api user`, or `git config user.name`); and appends a structural entry to the record's own revision log. It refuses outright, per-record, if that log doesn't exist, rather than writing somewhere it can't be audited. Rationale, alternatives, and every hand-authored sentence stay permanently off-limits: this is a cache of a computation, never an editor.

## Adopt without a rewrite

```sh
git clone https://github.com/urzuahq/urzua && cd urzua && make build
cd /path/to/your-repo
/path/to/urzua/rust/target/release/urzua init --dry-run # preview .urzua/config.toml from what's already there
/path/to/urzua/rust/target/release/urzua init           # writes it; never moves or rewrites a single record
/path/to/urzua/rust/target/release/urzua check docs/    # validates against it
```

`init` reads your existing directories and header conventions and proposes a config. It never renumbers, never rewrites, never moves a file. Nothing about your corpus changes until you accept what it found. Confirm this yourself rather than trust it: `make ci` in this repo runs the same `urzua check docs/` against Urzua's own corpus, and [`ci.yml`](.github/workflows/ci.yml) runs it on every push and pull request.

## The schema is declared, not assumed

A record type is whatever `.urzua/config.toml` says it is. Urzua ships no hardcoded notion of "an ADR" or "an RFC."

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

Every existing header convention this project has encountered is a declared shape, not a guess: a blockquote (`> Key: Value`), a bold-labelled blockquote, a pipe-delimited single line, a bold markdown list with no blockquote at all, and YAML frontmatter. A misdeclared shape fails loud (`no header-shaped region found`), never a silent match on the wrong lines.

This repo's own `.urzua/config.toml` declares `milestone` and `bug` alongside `adr`/`rfc`/`spec`: two record types added after the fact, with zero changes to `urzua-core`, to track this project's own backlog and defects using the same engine that validates its decisions.

## Lineage between record types is also declared, not assumed

Fields aren't just data — some of them are pointers to other records, and which fields mean that, and which direction they point, is a config-time choice, not something Urzua hardcodes any more than it hardcodes "an ADR."

A single record can point backward at more than one other record — `Implements`/`Derives-from` are comma-separated, so this is a graph, not a chain. This repo's own [`SPEC-8`](docs/specs/SPEC-8-urzua-fix.md) implements four ADRs at once (`Implements: ADR-15, ADR-18, ADR-19, ADR-20`, bundling four related decisions into one buildable reference), and nothing stops a spec from pointing straight at an RFC and skipping the ADR stage entirely, if that's how a given decision actually happened.

The simplest real path in this corpus still illustrates the shape clearly, so it's worth walking end to end:

```
RFC-1 (proposal)  <--Derives-from--  ADR-10 (decision)  <--Implements--  SPEC-16 (buildable spec)
```

Concretely: [`RFC-1`](docs/rfc/RFC-1-unified-record-schema-core-and-profiles.md) proposed a unified core-plus-profile schema. [`ADR-10`](docs/adr/ADR-10-unified-record-schema-core-and-profiles.md) decided it, and its header carries `Derives-from: RFC-1` — "I'm deciding the thing that RFC proposed." [`SPEC-16`](docs/specs/SPEC-16-the-adr-record-type.md) is the buildable detail of what ADR-10 decided, and carries `Implements: ADR-10` — "I'm the built-out reference for what that ADR decided." Each points backward at the thing it came from, never forward at what came later, since the later record usually doesn't exist yet when the earlier one is written.

`urzua check` resolves every one of these pointers against the real corpus (`pointer.resolution`): a typo'd or dangling reference is a finding, not a silent broken link. `urzua graph` dumps the whole thing as edges — every `Implements`/`Derives-from`/`Supersedes`/`Parent` reference across every record, each tagged `dangling: bool` — so the graph shape (who points at whom, and how many times) is a query, not something you infer by reading every file:

```
$ urzua graph
{"edges": [
  {"from": "SPEC-8",  "relation": "Implements",  "to": "ADR-15", "dangling": false},
  {"from": "SPEC-8",  "relation": "Implements",  "to": "ADR-18", "dangling": false},
  {"from": "SPEC-16", "relation": "Implements",  "to": "ADR-10", "dangling": false},
  {"from": "ADR-10",  "relation": "Derives-from", "to": "RFC-1", "dangling": false}
]}
```

None of `Implements`, `Derives-from`, or the graph's shape is special-cased in `urzua-core` — they're plain field names an org declares in `known_fields` per type, resolved generically by one rule that doesn't know or care what "RFC" or "ADR" means. A different org could:

- Skip the RFC stage entirely and decide directly (this repo's own `milestone`/`bug` types do exactly that — `ADR-34`/`ADR-35` with no RFC upstream of either).
- Point an `incident-review` type at a `runbook` type instead of an ADR at an RFC — same mechanism, different vocabulary.
- Add a field like `Feeds-into` that points *forward* instead of back, if that org's workflow genuinely needs to know downstream consumers from the source record — nothing in the engine assumes lineage only flows one direction, or one-to-one, only that whatever edges you declare get checked for resolution.

The chain above is one path through this repo's own graph, documented here because it's concrete — not a schema Urzua expects every adopter to reproduce.

## Status

**v0.1.0, self-hosting.** Real and validated against this repository's own `docs/` on every commit:

| Command | State |
|---|---|
| `check` | Real: full rule set above, always JSON on stdout (ADR-23) |
| `explain <path>` | Real: every record whose `Realized-by` names this file as evidence (ADR-24) |
| `graph` | Real: the full `Implements`/`Derives-from`/`Supersedes` relationship graph, as data, dangling edges flagged (ADR-24) |
| `init` | Adopt mode only: proposes config from an existing corpus, never moves files |
| `fix` | Detect mode (Embodiment, Tier 1) and apply mode, both real |
| `migrate ids` | Real: backfills a collision-free stable ID ([ULID](docs/adr/ADR-21-ulid-as-the-stable-id-encoding.md)) into every record lacking one |
| `migrate schema --report` | Real: previews which records lack a real value for a proposed new required field, before it's ever added to config |
| `doctor` | Real: reports on the tool's own configuration health, not record content |
| `new <type> [title]` | Real: fills a checked-in template (or synthesizes YAML frontmatter if none exists) with a fresh stable ID, never asking for a number ([ADR-27](docs/adr/ADR-27-urzua-new-fills-the-template-in-not-the-decision.md)) |
| `audit` | Real: supersession reciprocity and dangling cross-references, read-only ([ADR-30](docs/adr/ADR-30-urzua-audit-reuses-checks-rule-functions.md)) |
| `migrate schema --assist-waivers`/`--apply`, `export`, `import` | Not implemented yet: exit 2 with "not implemented yet" |

See [`CHANGELOG.md`](CHANGELOG.md) for what shipped when, and [`docs/rfc/`](docs/rfc/) / [`docs/adr/`](docs/adr/) for what's designed but not yet built, including the fuller claim-graph model (AND/OR composites, cross-record-shared claims) this release's Embodiment tracking deliberately ships a narrower slice of first.

## Maintenance

Released for use, actively maintained: this is a real tool being built in the open, not published only for reference. Issues and PRs are triaged. The schema and CLI contract may still change before `v1.0.0`; `CHANGELOG.md` calls out anything breaking. `CHANGELOG.md` itself is compiled from per-PR changesets (`.changeset/`, [ADR-29](docs/adr/ADR-29-changesets-via-knope-supersedes-lockstep-verification.md)). See [`CONTRIBUTING.md`](CONTRIBUTING.md) for the format.

Build with `make`, not `cargo` directly. `make build` / `make test` / `make ci` work without
knowing which languages are involved, though native commands work inside `rust/` too. `make help`
lists every target.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Telemetry

Urzua collects nothing and phones home never. It reads the files you point it at and writes to
stdout/stderr and the paths you configure. No network calls, no analytics, no crash reporting.

## License

[MIT](LICENSE).
