# Monorepo layout

Polyglot, organized by language at the root ([ADR-0004](../adr/0004-polyglot-monorepo-layout.md)).
Nothing language-specific lives at the repo root — no `Cargo.toml`, no `package.json`.

| Directory | Purpose |
|---|---|
| `rust/` | Cargo workspace. `urzua-core` (pure schema + validation, no I/O), `urzua-id` (stable identifiers, [ADR-0003](../adr/0003-stable-record-identifiers.md)), `urzua-agdr` (AgDR export, [ADR-0002](../adr/0002-own-the-record-format-agdr-compatible-on-export.md)), `urzua-cli` → the `urzua` binary. |
| `ts/` | Reserved for agent-harness integrations (hooks/plugins/skills for Claude Code, Cursor, etc.) and a future dashboard. Empty until there's real content — see `ts/README.md`. |
| `platform/` | Reserved for deployment targets. Nothing is hosted yet; hosting needs its own ADR first. |
| `docs/adr/` | Decisions made about Urzua itself — accepted, load-bearing. |
| `docs/rfc/` | Pre-decision proposals — read these first for design rationale. |
| `docs/specs/` | Build-level detail for what actually ships. |
| `docs/reference/` | This file and other internal engineering docs — added only once they earn their place. |
| `.git-hooks/` | Opt-in local hooks — `make hooks-install` wires them into `.git/hooks/`. |

Build with `make`, not `cargo`, directly — `make ci` runs exactly what CI runs. `make help` lists
every target. Native `cargo` commands work fine inside `rust/` for anyone who prefers them.
