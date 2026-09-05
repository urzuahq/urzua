# 0004 — Polyglot monorepo, organized by language at the root

> Status: Accepted
> Embodiment: Specified
> Date: 2026-07-29
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —

## Context

SPEC-0001 assumed a single Cargo workspace at the repo root, implicitly treating Urzua as a
Rust project. That assumption is wrong, and it would have been expensive to unwind later.

ADR-0001 chose Rust for the CLI on distribution and hook-path startup grounds. Neither argument
generalizes past the CLI. Work already visible on the roadmap is not Rust-shaped:

- **Agent-harness integrations** — hooks, plugins, and skills for Claude Code, Cursor, Copilot,
  Codex, and Windsurf live in the TypeScript/Node ecosystem those harnesses
  are written for. Shipping a Rust binary to invoke from a Node plugin is fine; writing the plugin
  in Rust is not an option.
- **Dashboard** is a web application.
- **Integrations** with Jira, Linear, Slack follow their SDKs, which are overwhelmingly
  TypeScript.
- **Deployment and infrastructure**, once anything is hosted, is neither.

Forcing these into a Rust workspace, or scattering them into sibling repositories, both fail. The
first is impossible; the second reintroduces exactly the fragmentation this project exists to
eliminate — governance records, schema, and tooling drifting apart across repos is the founding
failure mode.

This shape is not novel: language directories at the root (`ts/`, `py/`, `rust/`, ...), a
`platform/` directory for deployment targets, shared `docs/` at the root, and a single root
`Makefile` dispatching into each is a common, well-worn pattern for a polyglot monorepo, chosen
here for exactly the reason it's common — it keeps governance records language-agnostic while
letting each language's own tooling work unmodified inside its own directory.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Cargo workspace at root** | Simplest for the CLI; idiomatic Rust | Cannot house TS or infra work without ugly nesting; forces a second repo the moment integrations start; assumes the project is a Rust project rather than a product with a Rust component |
| **Language folders at root** *(chosen)* | Each ecosystem gets its idiomatic layout; new languages are additive; one repo keeps schema, records, and tooling together; matches working prior art | Root-level dispatch needed so contributors don't learn N toolchains; language boundary can be mistaken for a module boundary |
| **Domain folders at root** (`validator/`, `backfill/`) | Organized by product concept | Mixes toolchains inside a directory; no ecosystem has a convention for it; build tooling fights it |
| **Separate repos per language** | Clean toolchain isolation | Fragmentation is the founding failure mode; cross-language schema changes become multi-repo migrations |

## Decision

In the context of a product whose CLI is Rust but whose integrations, dashboard, and infrastructure
will not be, facing a spec that wrongly assumed a Rust-rooted workspace, we decided to organize the
monorepo by **language at the root** with a single `Makefile` dispatching into each, to let every
ecosystem use its native layout while keeping schema, records, and tooling in one repository,
accepting that contributors must go through the root dispatch rather than each language's native
commands.

Reserved top-level names:

| Directory | Contents |
|---|---|
| `rust/` | Cargo workspace. The `urzua` CLI and its crates. |
| `ts/` | TypeScript/Node. Agent-harness integrations, editor plugins, dashboard. |
| `platform/` | Deployment targets and infrastructure, per environment. |
| `docs/` | Governance records — RFCs, ADRs, specs. Language-agnostic by nature; the record format is the shared contract every language directory implements against. |
| `corpora/` | Synthetic acceptance-test fixtures (SPEC-0004). |

Rules:

1. **Nothing language-specific at the root.** No `Cargo.toml`, no `package.json`, no
   `pyproject.toml`. The root holds the `Makefile`, `README.md`, `CLAUDE.md`, `.github/`, and the
   language directories.
2. **`docs/` stays at the root, not inside a language directory.** Records govern the whole project
   and are consumed by every language directory.
3. **The root `Makefile` is the contract.** `make check`, `make test`, `make build` work without
   knowing which languages are involved. Each language directory owns its own dispatch beneath that.
4. **Language directories are not module boundaries.** `rust/` is not a layer; it's a toolchain.
   Product structure lives in the crates and packages inside it.
5. **New top-level directories require an ADR amendment.** The list is short deliberately; the
   failure mode of polyglot monorepos is root-level sprawl.

## Reversibility

**Highly reversible now; progressively less so.** With one language present and no published
artifacts, this is a directory move. It gets harder once CI, editor plugins, release automation,
and external documentation reference paths — but even then it's mechanical, not architectural.

Deciding it now rather than after the CLI exists is what keeps it cheap: this ADR corrects SPEC-0001
before any code was written against the wrong assumption, which is the whole argument for recording
decisions before building.

## Consequences

- SPEC-0001's monorepo layout section is **substantively wrong** and must be amended, not quietly
  edited. Recorded as a substantive revision per RFC-0001 §4.
- Contributors run `make`, not `cargo`, as the entry point. Native commands still work inside each
  language directory for anyone who prefers them.
- CI needs per-language jobs with path filters, so a docs-only change doesn't trigger a
  cross-compilation matrix.
- `rust/` carries a nesting level of overhead for what is currently the only real code. Accepted
  deliberately: the cost is one directory, and the alternative is a repo split later.
- Release artifacts come from `rust/`, so release automation must not assume a root workspace.
- The structure invites premature creation of empty directories. Per the project's existing
  convention, a directory appears when it has real content — `ts/` and `platform/` carry only a
  README stating what belongs there until they hold something.

## References

- `docs/specs/0001-v0-cli.md` — amended by this ADR
- ADR-0001 — Rust for the CLI, on grounds that don't generalize past the CLI
- Prior art: a polyglot monorepo layout with root-level language directories
