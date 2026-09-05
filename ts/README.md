# `ts/` — TypeScript / Node

Reserved per [ADR-0004](../docs/adr/0004-polyglot-monorepo-layout.md). Empty of code for now; this
README exists so the convention is discoverable rather than tribal.

## What belongs here

Work in the TypeScript/Node ecosystem — chosen because that's where the target ecosystems live, not
by preference:

- **Agent-harness integrations** — hooks, plugins, and skills for Claude Code, Cursor, Copilot,
  Codex, Windsurf. These have to be written in the language their harness expects; shipping a Rust
  binary for them to *invoke* is fine, writing the plugin itself in Rust is not an option.
- **Dashboard** — a web application if and when one exists.
- **Integrations** — Jira, Linear, Slack, whose SDKs are overwhelmingly TypeScript.

## What does not belong here

- The `urzua` CLI and its logic. That's Rust ([ADR-0001](../docs/adr/0001-rust-as-implementation-language.md)),
  and it lives in [`rust/`](../rust).
- Governance records. Those stay in [`docs/`](../docs) at the root — they govern the whole project,
  not one toolchain.
- **A reimplementation of the validator.** If validation logic ends up written twice, the founding
  claim of this project — build it once instead of three times — has been falsified inside its own
  repo. TypeScript code calls the binary; it does not reproduce it.
