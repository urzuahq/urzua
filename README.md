# Urzua

A CLI that validates decision records (RFCs, ADRs, specs) as a structured, closed schema — header
shape, cross-reference reciprocity, realization tracking — instead of leaving each rule to a
one-off regex.

## Status

**v0.1.0, self-hosting.** `urzua check` and `urzua init` (adopt mode) are real and validate this
repository's own `docs/` on every run. `new`, `audit`, `migrate`, `export`, and `import` still exit
2 with "not implemented yet" — see [`CHANGELOG.md`](CHANGELOG.md) for exactly what's built versus
stubbed, and [`docs/specs/`](docs/specs/) for what's designed but not yet built.

## Install

Download a release binary for your platform from the
[latest release](https://github.com/urzuahq/urzua/releases/latest), or build from source:

```sh
git clone https://github.com/urzuahq/urzua && cd urzua && make build
```

The binary lands at `rust/target/release/urzua`.

## Maintenance

Released for use, actively maintained — this is a real tool being built in the open, not published
only for reference. Issues and PRs are triaged. The schema and CLI contract may still change before
`v1.0.0`; `CHANGELOG.md` will call out anything breaking.

## Why

A decision record (an ADR, an RFC, a spec) is only useful if its shape is dependable: every header
field present and unambiguous, every cross-reference (`Supersedes`, `Implements`, `Related`)
resolving to something real, every claim of "this was built" backed by evidence a tool can check
rather than prose someone remembers to update. Hand-written linters for this tend to check fields
one at a time and miss the header as a *structure* — a bounded region with a known key set, each
key appearing exactly once, nothing else. Urzua treats the header as a closed structure with one
shared parser, validates cross-references bidirectionally, and tracks whether a decision was
actually realized (`Not started → Specified → Implemented → Verified`, plus `Drift detected`)
independently of whether it was *decided*.

The design record for all of this — what's settled, what's still open, and why — lives in
[`docs/rfc/`](docs/rfc/), [`docs/adr/`](docs/adr/), and [`docs/specs/`](docs/specs/).

## Layout

Polyglot monorepo, organized by language at the root ([ADR-0004](docs/adr/0004-polyglot-monorepo-layout.md)):

```
rust/           Cargo workspace — urzua-core (pure schema+validation), urzua-id
                (stable identifiers), urzua-agdr (AgDR export format), urzua-cli
                (the `urzua` binary)
ts/             Reserved for agent-harness integrations. Empty until there's real content.
platform/       Reserved for deployment targets. Empty until there's real content.
docs/adr/       Decisions made about Urzua itself.
docs/rfc/       Pre-decision proposals — the design record, read these first.
docs/specs/     Build-level detail for what ships.
```

Build with `make`, not `cargo`, directly — `make build` / `make test` / `make check` work without
knowing which languages are involved, though native commands work inside `rust/` too. `make help`
lists every target.

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md).

## Telemetry

Urzua collects nothing and phones home never. It reads the files you point it at and writes to
stdout/stderr and the paths you configure. No network calls, no analytics, no crash reporting.

## License

See [`LICENSE`](LICENSE).
