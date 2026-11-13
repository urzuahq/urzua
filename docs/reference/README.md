# Urzua user documentation

These pages are for people who run `urzua` on their own repository. They describe the tool as it is
on the `main` branch of this repository. Every command, flag, config key and output field named here
exists in that code; when a page says a feature is not available yet, it is not.

Read them in this order the first time:

| Page | What it covers |
|---|---|
| [Install](install.md) | Getting a binary, and which release matches these pages |
| [Adopt an existing corpus](adopt.md) | `init`, the first `check`, and the first week of using urzua |
| [The record model](record-model.md) | Record types, headers, statuses, relations and evidence |
| [The config file](config.md) | Every key of `.urzua/config.yaml` |
| [`check` and its JSON output](check.md) | `check`, `check --base`, exit codes and every output field |
| [Command reference](commands.md) | Every command and flag |
| [Waivers and drift](waivers-and-drift.md) | Reviewed exceptions, evidence pins, `fix`, `reverify` |
| [CI](ci.md) | The gate action and the stale-pin step |
| [The agent harness](agent-harness.md) | Hooks that make an AI agent's edits go through urzua |
| [The MCP server](mcp.md) | `urzua mcp`: a read-only server that lets an agent read records, findings and relations |

Two things to know before you start:

- **Every command prints one JSON object on stdout, and nothing else.** There is no human-readable
  mode. Prompts, notices and progress go to stderr. Pipe stdout to `jq` or to your own program.
- **Nothing runs unless you declare it.** A rule that is not named under `rules` in the config does
  not run, and `check` lists it as `not-enabled`. `init` writes a starter config so you begin with a
  working set.

For the design reasoning behind a behaviour, see the decision records in [`../adr/`](../adr/) and the
build-level specifications in [`../specs/`](../specs/). Those are written for the people building
urzua; these pages are written for the people using it. For what changed in each release, see
[`CHANGELOG.md`](../../CHANGELOG.md).
