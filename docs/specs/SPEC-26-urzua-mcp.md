---
Stable-Id: 01M7A8QGWXMR8WYFDC36XW8XEF
Status: Draft
Date: 2026-11-13
Version: '0.6'
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
  configuration, a usage error such as an unknown flag or `--as-of`); the failure is one JSON notice on stderr and
  nothing is written to stdout.
- The root is `--root` when given, else the repository holding the directory the server was started in, found as every
  command finds it. The engine reads no agent-vendor environment variable; a client that starts servers from another
  directory passes `--root`.
- The configuration is read for every call, so an edit to a record or to `.urzua/config.yaml` is seen by the next call.
- No other flag is accepted. In particular there is no flag that enables a write, a format or a listener.

## Transport

stdio only. A frame is one JSON-RPC 2.0 message on one line, terminated by a newline, with no embedded newline. Stdout carries
frames and nothing else; every line a client reads from it parses as a JSON-RPC message. A request line is read up to 1 MiB
(1,048,576 bytes, newline excluded); a longer one is answered with `-32600` and skipped to its newline without being held in
memory, and a line that is not UTF-8 is answered with `-32700`. The next line is served in both cases. Notices go to stderr as JSON, one per
line. The pinned protocol revision is `2025-06-18`. The server answers `initialize` with that revision whatever the client
offered; a client that cannot speak it ends the session. Streamable HTTP is not built.

Methods served: `initialize`, `notifications/initialized`, `ping`, `tools/list` and `tools/call`. An unknown method is the JSON-RPC
"method not found" error (`-32601`); malformed JSON is `-32700`; a request that is not a valid message is `-32600`; invalid
parameters are `-32602`. A message without an `id` is a notification: it is never answered and never runs a tool, `tools/call` included. `id` is a string or a
number; `null` or any other type is `-32600` (the MCP rule, stricter than JSON-RPC). A `notifications/*` method sent with an `id`
is `-32600`. Capabilities advertised: `tools` only.

## Tools

Every tool is advertised with `readOnlyHint: true` and an input schema that sets `additionalProperties: false`. An argument the
schema does not name is `-32602`, never ignored. Names carry a read word and no word the harness reads as a write (`write`,
`create`, `set`, `update`, `move`, `delete`, `apply`, `fix`), so the harness leaves a call to them alone (`SPEC-19`).

| Tool | Input | Result |
|---|---|---|
| `urzua_check` | `{paths?: [string], base?: string, rules?: [string]}` | `CheckReport` |
| `urzua_list_records` | `{type?: string, status?: [string]}` | `ExportReport` with each record's `raw_text`, `sections` and `preamble` left out: `types`, `record_count` and `records[]` (id, type, path, header fields). `types[].records` and `record_count` count the records listed, not the corpus. The `roadmap` block `export` carries is left out: it spans the whole corpus whatever the filter, so a two-record listing would carry the whole roadmap. A roadmap tool stays unbuilt (below); `urzua roadmap` is the command. |
| `urzua_show_record` | `{id: string}` | one `ExportedRecord` (header, `sections`, `raw_text`) and the `GraphEdge`s that touch it, supersession included |
| `urzua_get_findings_for_path` | `{path: string}` | `ExplainReport` |
| `urzua_get_graph` | `{}` | `GraphReport` |

`urzua_check` runs `check` in process over the corpus. It has no equivalent of `--stdin-path`, `--actors`, `fix` or any
`--apply`. `urzua_list_records` filters by the record type when `type` is given and by the header's status value when `status`
is given; the status names are compared as declared, and the server knows no status literal (`ADR-61`). A `type` or a `status`
that matches nothing is a notice in the result (`list-records.type`, `list-records.status`), never silently ignored. Rejected and superseded decisions are
`{type: "adr", status: ["Rejected", "Superseded"]}` in a corpus that declares those statuses. `urzua_show_record` returns the
record's sections as written; the reasons for a decision are the prose of its sections and are not parsed (`ADR-64`).

All five tools are built, and `tools/list` lists exactly them. Each name carries a read word and no write word, because the editor
hook leaves an MCP call alone only by its name (`SPEC-19`): `urzua_findings_for_path` and `urzua_graph` were named without one and
were judged as edits when given a gated path, so they are `urzua_get_findings_for_path` and `urzua_get_graph`.
`urzua_show_record` returns `{status, record, edges, notices}`: the `ExportedRecord` and the `GraphEdge`s whose `from` or `to` is
the id (`ADR-46` has no report for this, so it is those two existing types in one object); an id no record carries is a
could-not-run result. `urzua_get_findings_for_path` refuses a `path` that is absolute or contains `..`, as `urzua_check` refuses a `paths` entry that is absolute or
contains `..` (`-32602`): the tools read the repository, not the disk around it.

## Results and errors

A successful `tools/call` result carries the report as `structuredContent` and the same JSON, serialized once, as the single text
block in `content`. `isError` is `false`. A report that says the repository has findings is still a success: whether the
repository is clean is the report's answer, as for the CLI.

A failure the command line reports as a could-not-run (an unreadable corpus, an unknown id, an unknown rule) is a result with
`isError: true`, the `CouldNotRun` shape of `ADR-46` as `structuredContent` and as the text block. A report whose `status` is `not-run` is an error result too, whatever built it (a `urzua_check` whose `paths` name no record, as the command exits 2): `isError` is `true` exactly when `status` is `not-run`. A report with findings is `isError: false`; its `blocking` is true exactly when the command line would exit 1, `fail_on` included, because a client has no exit code to read. An unknown tool name is the
JSON-RPC error `-32602` and is never forwarded. A result that examined no records says so in the report's `population` or
`files_examined`; a tool never returns an empty report for a corpus it could not read.

## Read-only guarantee

The server has no code that writes. It builds each report through the function the matching command uses, takes no write handle,
runs no shell and never calls `urzua` as a child. It does start `git`, through the engine's one git runner in `urzua-io`, and only
with verbs that read the repository (listing files, comparing against a base, blame, log): that is how `check` reads the corpus.
The mutating commands (`new`, `fix --apply`, `index --apply`, `migrate`,
`reverify`, `init`, `waive --apply`, `harness install` and `harness update`) have no tool. The tests that prove it:

- a call to an unknown or mutating name (`urzua_write_record`, `urzua_fix_apply`) is refused and the tree is unchanged;
- the working tree and `.git`, every file and every directory (an empty one included), hash the same before and after every tool, in a
  table of every tool;
- a scan of the production code of the `urzua-mcp` crate, the `mcp` command and the `urzua-cli` files its tools run through
  (`commands/check.rs`, `commands/export.rs`, `commands/explain.rs`, `commands/graph.rs`, `session.rs`, `discovery.rs`, `file_texts.rs`, `diff_scope.rs`) finds no write API
  (`fs::write`, `rename`, `remove_*`, `create_dir*`, `File::create`, every `OpenOptions` spelling including `File::options()` and its
  `write`, `append`, `create`, `create_new` and `truncate` switches, `write_atomic`, process spawning). A `#[cfg(test)]` item hides
  only itself, and planted sources show the scan can fail (`ADR-55`). The scan does not cover `urzua-io` or `urzua-core`: `urzua-io`
  also holds the writers other commands use, so what the tools reach there is covered by the next item, not by the scan;
- every stdout line is a JSON-RPC frame;
- every tool name the real server advertises carries a read word and no write word, the harness leaves each
  `mcp__urzua__<tool>` alone when given a gated path, and judges a planted `mcp__urzua__urzua_write_record` as a write.

The editor hook is advisory for agents (`SPEC-19`); the guarantee is the absence of write code, and the tests above are what keep it.
One thing outside the server's code can change `.git`, and the guarantee is worded to allow it: `urzua_check` with `base` runs `git diff`, and git refreshes its own index
whenever a tracked file's stat data no longer matches the index (a touched file, a checkout that changed an mtime, a file written in the same
instant as the index: "racily clean"), which rewrites `.git/index` and takes `.git/index.lock` for the moment. That is git refreshing its cache, never a
record or a working file; a client that holds `index.lock` itself sees the call wait or fail; the tree-hash test starts from a settled index so it does not depend on timing. The guarantee is that
the server writes no record, no working file and no git object or ref; it does not promise that git leaves its own index alone.

## Registration

The server is registered by a person in a user-owned file, with the snippet documented in `docs/reference/mcp.md`, the README and `AGENTS.md`:

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

- `ADR-99` (this server's shape, approved by the maintainer in conversation on 2026-11-13; its Status moves to Accepted in a follow-up the maintainer approves), `ADR-46` (the report contract), `ADR-61` (declared status names), `ADR-64`, `ADR-89` (the
  harness and root resolution), `ADR-55`, `RFC-27`, `SPEC-13` (`explain` and `graph`), `SPEC-19`, `SPEC-24`, `MILE-29`, `MILE-47`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-13 | Initial spec; nothing built yet. **Why:** `ADR-99` decides the read-only MCP server and requires its tool contract to be specified before the code. | **structural** |
> | 2026-11-13 | Bumped to `0.2`. `urzua mcp` is built with `urzua_check` and `urzua_list_records` (`rust/crates/urzua-mcp`, `commands/mcp.rs`); the other three tools, the harness interaction tests and the registration docs follow. A listing leaves out `sections` and `preamble` as well as `raw_text`; `paths` may not leave the repository. **Why:** the server core of `MILE-29`. | **substantive** |
> | 2026-11-13 | Bumped to `0.3`. A request line is capped at 1 MiB and a non-UTF-8 line is answered, both without ending the session; `id` null is refused and a message without an `id` never runs a tool; the read-only guarantee no longer says the server spawns no process (it runs read-only `git`, which may refresh its own index) and states what the source scan covers. **Why:** the review of the server core. | **substantive** |
> | 2026-11-13 | Bumped to `0.4`. `urzua_show_record`, `urzua_get_findings_for_path` and `urzua_get_graph` are built; the last two are renamed from `urzua_findings_for_path` and `urzua_graph` so each name carries a read word the editor hook recognises; the harness interaction tests and the registration documentation exist. **Why:** the second change of `MILE-29`; the hook test showed the first names were judged as edits. | **substantive** |
> | 2026-11-14 | Bumped to `0.5`. A `not-run` report is an error result (`isError: true`); `urzua_list_records` leaves out the whole-corpus `roadmap` block; the `.git/index` note says git refreshes the index for any stat-dirty tracked file and may hold `index.lock`, not only for a racily clean one. **Why:** the external review's findings 4.3, 4.4 and 4.5; `ADR-46` says a result that established nothing is not a success. | **substantive** |
> | 2026-11-14 | Bumped to `0.6`. A usage error of `urzua mcp` (an unknown flag or argument, `--as-of`) is one JSON object on stderr and nothing on stdout, as the command section already said. **Why:** an external review found the refusal printed on stdout, where a client expects frames only. | **substantive** |
