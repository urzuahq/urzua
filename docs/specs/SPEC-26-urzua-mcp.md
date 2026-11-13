---
Stable-Id: 01M7A8QGWXMR8WYFDC36XW8XEF
Status: Draft
Date: 2026-11-13
Version: '0.2'
Author: beauwilliams
Subject: '`urzua mcp` -- a read-only MCP server over stdio that serves the reports urzua already builds.'
Implements: ADR-99
Parent: SPEC-1
---
# SPEC-26 — urzua mcp

## Purpose

`urzua mcp` runs a Model Context Protocol server on stdin and stdout so an agent can read the corpus's records, findings,
governing records and relations without a shell. It is a transport, not a capability: each tool returns a report the CLI already
builds, serialized unchanged (`ADR-99`, `ADR-46`). It has no tool that writes. This spec is the reference for the command and its
tool contract; `MILE-29` tracks what of it is built.

## Command

```
urzua mcp [--root DIR] [--config FILE]
```

- The process serves until stdin closes, then exits `0`. It exits `2` when it could not start (no configuration, an unreadable
  configuration); the failure is one JSON notice on stderr and nothing is written to stdout.
- The root is `--root` when given, else resolved as the harness settings command resolves it (`ADR-89`): `CLAUDE_PROJECT_DIR`, else
  the nearest real working tree holding `.urzua/config.yaml`. It is never the server's own directory by accident.
- The configuration is read for every call, so an edit to a record or to `.urzua/config.yaml` is seen by the next call.
- No other flag is accepted. In particular there is no flag that enables a write, a format or a listener.

## Transport

stdio only. A frame is one JSON-RPC 2.0 message on one line, terminated by a newline, with no embedded newline. Stdout carries
frames and nothing else; every line a client reads from it parses as a JSON-RPC message. Notices go to stderr as JSON, one per
line. The pinned protocol revision is `2025-06-18`. The server answers `initialize` with that revision whatever the client
offered; a client that cannot speak it ends the session. Streamable HTTP is not built.

Methods served: `initialize`, `notifications/initialized`, `ping`, `tools/list` and `tools/call`. An unknown method is the JSON-RPC
"method not found" error (`-32601`); malformed JSON is `-32700`; a request that is not a valid message is `-32600`; invalid
parameters are `-32602`. A notification is never answered. Capabilities advertised: `tools` only.

## Tools

Every tool is advertised with `readOnlyHint: true` and an input schema that sets `additionalProperties: false`. An argument the
schema does not name is `-32602`, never ignored. Names carry a read word and no word the harness reads as a write (`write`,
`create`, `set`, `update`, `move`, `delete`, `apply`, `fix`), so the harness leaves a call to them alone (`SPEC-19`).

| Tool | Input | Result |
|---|---|---|
| `urzua_check` | `{paths?: [string], base?: string, rules?: [string]}` | `CheckReport` |
| `urzua_list_records` | `{type?: string, status?: [string]}` | `ExportReport` with each record's `raw_text`, `sections` and `preamble` left out: `types`, `record_count` and `records[]` (id, type, path, header fields). `types[].records` and `record_count` count the records listed, not the corpus. |
| `urzua_show_record` | `{id: string}` | one `ExportedRecord` (header, `sections`, `raw_text`) and the `GraphEdge`s that touch it, supersession included |
| `urzua_findings_for_path` | `{path: string}` | `ExplainReport` |
| `urzua_graph` | `{}` | `GraphReport` |

`urzua_check` runs `check` in process over the corpus. It has no equivalent of `--stdin-path`, `--actors`, `fix` or any
`--apply`. `urzua_list_records` filters by the record type when `type` is given and by the header's status value when `status`
is given; the status names are compared as declared, and the server knows no status literal (`ADR-61`). A `type` or a `status`
that matches nothing is a notice in the result (`list-records.type`, `list-records.status`), never silently ignored. Rejected and superseded decisions are
`{type: "adr", status: ["Rejected", "Superseded"]}` in a corpus that declares those statuses. `urzua_show_record` returns the
record's sections as written; the reasons for a decision are the prose of its sections and are not parsed (`ADR-64`).

Built today: `urzua_check` and `urzua_list_records`. `urzua_show_record`, `urzua_findings_for_path` and `urzua_graph` follow
and are not served yet. `tools/list` lists exactly the tools built. `urzua_check` refuses a `paths` entry that is absolute or
contains `..` (`-32602`): the tools read the repository, not the disk around it.

## Results and errors

A successful `tools/call` result carries the report as `structuredContent` and the same JSON, serialized once, as the single text
block in `content`. `isError` is `false`. A report that says the repository has findings is still a success: whether the
repository is clean is the report's answer, as for the CLI.

A failure the command line reports as a could-not-run (an unreadable corpus, an unknown id, an unknown rule) is a result with
`isError: true`, the `CouldNotRun` shape of `ADR-46` as `structuredContent` and as the text block. An unknown tool name is the
JSON-RPC error `-32602` and is never forwarded. A result that examined no records says so in the report's `population` or
`files_examined`; a tool never returns an empty report for a corpus it could not read.

## Read-only guarantee

The server has no code that writes. It builds each report through the function the matching command uses, takes no write handle,
spawns no process, and never calls `urzua` as a child. The mutating commands (`new`, `fix --apply`, `index --apply`, `migrate`,
`reverify`, `init`, `waive --apply`, `harness install` and `harness update`) have no tool. The tests that prove it:

- a call to an unknown or mutating name (`urzua_write_record`, `urzua_fix_apply`) is refused and the tree is unchanged;
- the working tree and `.git` hash the same before and after every tool, in a table of every tool;
- a scan of the `urzua-mcp` crate and the `mcp` command module finds no write API (`fs::write`, `rename`, `remove_*`,
  `OpenOptions`, `File::create`, process spawning), and a planted file shows the scan can fail (`ADR-55`);
- every stdout line is a JSON-RPC frame;
- the harness leaves `mcp__urzua__urzua_list_records` with a `path` argument alone and judges a planted
  `mcp__urzua__urzua_write_record` as a write.

The editor hook is advisory for agents (`SPEC-19`); the guarantee is the absence of write code, and the tests above are what keep it.

## Registration

The server is registered by a person in a user-owned file, with a documented snippet in the README and `AGENTS.md`:

```json
{ "mcpServers": { "urzua": { "command": "urzua", "args": ["mcp"] } } }
```

`urzua harness install` and `urzua harness update` write no MCP configuration, so the protected `.claude` surface is not touched.

## What's deliberately not built

- Streamable HTTP, a listener and authentication: a hosted-service shape `RFC-27` rules out (`ADR-99`).
- Resources, prompts, sampling, logging and list-changed notifications; only tools are advertised.
- Any tool that writes, including one that applies a fix, creates a record or installs the harness.
- A dedicated rejected-decisions tool: that is a query of `urzua_list_records`.
- A status or roadmap tool (`urzua_status`): it would duplicate `doctor` and `roadmap`; added only on a recorded need.
- A second output shape, a text-only mode or a format option (`ADR-23`, `ADR-26`).
- An SDK dependency; adopting one needs an ADR amending `ADR-1` (`ADR-99`).

## References

- `ADR-99` (this server's shape), `ADR-46` (the report contract), `ADR-61` (declared status names), `ADR-64`, `ADR-89` (the
  harness and root resolution), `ADR-55`, `RFC-27`, `SPEC-13` (`explain` and `graph`), `SPEC-19`, `SPEC-24`, `MILE-29`, `MILE-47`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-13 | Initial spec; nothing built yet. **Why:** `ADR-99` accepted the read-only MCP server and requires its tool contract to be specified before the code. | **structural** |
> | 2026-11-13 | Bumped to `0.2`. `urzua mcp` is built with `urzua_check` and `urzua_list_records` (`rust/crates/urzua-mcp`, `commands/mcp.rs`); the other three tools, the harness interaction tests and the registration docs follow. A listing leaves out `sections` and `preamble` as well as `raw_text`; `paths` may not leave the repository. **Why:** the server core of `MILE-29`. | **substantive** |
