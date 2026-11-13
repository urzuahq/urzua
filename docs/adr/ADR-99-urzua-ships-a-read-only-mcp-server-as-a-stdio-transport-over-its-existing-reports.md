---
Stable-Id: 01M7A8QFEVHGZ559S89JSYR8TA
Status: Accepted
Accepted-on: 2026-11-13
Date: 2026-11-13
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
---
# 99 — urzua ships a read-only MCP server as a stdio transport over its existing reports

## Context

`MILE-29` names "a read-only MCP server" as a build target and `MILE-47` asks for the position to be decided first. No ADR or
RFC mentions MCP: `RFC-27` lists it only as an open question, whether it is "engine or ecosystem", and notes that it carries no
new capability, only a way for an agent to reach what `check`, `explain` and `graph` already report. An agent that wants to know
what was already rejected or superseded today has to run Bash and read the corpus; the harness (`ADR-89`) already matches
`mcp__.*` tool calls and judges them by name, so an MCP tool is a surface the repository already governs.

The constraints are already decided elsewhere:

- Stdout is JSON and nothing else, with no format flag and no second rendering (`ADR-23`, `ADR-26`), and every command's output is
  one `Report` through `emit` (`ADR-46`). A tool result must not invent a second shape.
- `urzua-core` is pure and I/O lives in `urzua-io` (`ADR-5`); the binary is one static artifact that adds nothing to the host
  (`ADR-1`, `ADR-13`). `deny.toml` admits only MIT, Apache and Unicode licences and warns on duplicate versions.
- Structured in, structured out; prose only in sections (`ADR-64`). Status names are declared, never hardcoded (`ADR-61`).
- A claim of "cannot write" must be proved by a test that fails on a planted violation (`ADR-55`).

The data a rejected or superseded decision carries is already in `urzua export` (header fields, sections, raw text) and the
supersession chain is already in `urzua graph`. The "reasons" are prose in the record's sections; the server returns them as
sections and never parses their meaning.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Ship in the engine as a read transport over existing reports | No new capability, so nothing to specify twice; one binary and one config loader; opt-in by registration | The engine gains a protocol to track |
| B. Ship as ecosystem (a separate crate or feature flag outside the default build) | Keeps the protocol out of the default build | A second artifact to distribute and version for no isolation gain; the server would drift from the reports it carries |
| C. Do not ship; agents shell out to `urzua export` and `check` | Nothing to build | Every agent needs Bash and a corpus path to read a rejected decision; the editor hook cannot tell a read from a write by tool name |
| D. Data scope: only rejected and superseded decisions | The milestone's literal text | Hardcodes status literals (against `ADR-61`) and answers one question only |
| E. Data scope: every record, with a generic `status` filter | General; "rejected and superseded" is one query | The caller names the statuses |
| F. Data scope: a dedicated `rejected_decisions` tool | Easiest call | Hardcodes type and status knowledge in the engine |
| G. Transport: stdio only | No listener, no authentication, no hosted-service shape (which `RFC-27` rules out) | Local use only |
| H. Transport: stdio plus Streamable HTTP, or HTTP only | Remote use | A listener, authentication and a service to operate |
| I. Start with a `urzua mcp` subcommand | One binary, shared config loading and `emit`, one release artifact | None of note |
| J. Start with a separate `urzua-mcp` binary, or a harness-installed launcher script | Isolation | A second artifact to distribute (`ADR-13`); a launcher is a script to install and protect |
| K. Protocol: hand-written minimal JSON-RPC in a new `urzua-mcp` crate | Zero new dependencies; synchronous; no change to `deny.toml` or the purity allowlist; protocol layer testable by feeding frames | We track one pinned protocol revision ourselves |
| L. Protocol: an official SDK crate | Less protocol code | An async runtime and a large transitive tree in a tool whose stated cost is startup (`ADR-1`); `deny` and licence review; protocol churn |
| M. Result shape: the report JSON as `structuredContent` plus an identical text block | No second shape (`ADR-46`); works for clients that read either | The JSON is carried twice |
| N. Result shape: text only, or an MCP-specific envelope | Simpler, or tailored | Loses the typed result, or adds a second shape |
| O. Registration: a documented snippet; `harness install` writes no MCP config | Opt-in; does not touch the protected `.claude` surface | A person pastes a snippet |
| P. Registration: `harness install --mcp`, or automatic | No manual step | The harness would write a user-owned file the repository does not govern |

## Decision

*In the context of agents needing the corpus's rejected and superseded decisions without a shell, facing a protocol the engine
has no record of and a one-binary, minimal-dependency, JSON-only discipline, we decided that the maintainer's choices of
2026-11-13 stand as follows, to achieve a read-only reader that adds no capability and no second output shape, accepting that we
track one protocol revision ourselves:*

1. **MCP ships in the engine as a transport over existing reports.** The test that settles `RFC-27`'s open question: a transport,
   not a capability, is engine. The server builds the same report each command builds, by calling the same function, never by
   running `urzua` as a child process or capturing its stdout.
2. **A generic status filter.** `urzua_list_records` takes an optional type and an optional list of statuses. No status literal
   is known to the server; rejected and superseded decisions are the query `status: ["Rejected", "Superseded"]` (`ADR-61`).
3. **stdio only.** Newline-delimited JSON-RPC 2.0 on stdin and stdout, one pinned protocol revision named in the spec. Stdout
   carries protocol frames and nothing else; notices go to stderr as JSON. Streamable HTTP is a recorded non-goal.
4. **A `urzua mcp` subcommand**, specified in `SPEC-26`. The root is resolved as the harness settings command resolves it
   (`ADR-89`), so a server started from another directory does not read its own.
5. **Hand-written minimal JSON-RPC in a new `urzua-mcp` crate.** The crate holds framing and dispatch over `Read` and `Write`
   and touches no filesystem; the report-producing calls are injected by `urzua-cli`. Zero new dependencies; no change to
   `deny.toml` or to the purity allowlist of `urzua-core`. Adopting an SDK later needs its own ADR amending `ADR-1` and a
   dependency budget.
6. **The report JSON is the result.** Each tool returns the report it claims as `structuredContent` and the identical JSON as one
   text block. A failure is the `CouldNotRun` shape of `ADR-46` with `isError: true`.
7. **Registration is a documented snippet.** `urzua harness install` writes no MCP configuration. Registering the server is a
   person's act in a user-owned file.

The tools are `urzua_check`, `urzua_list_records`, `urzua_show_record`, `urzua_findings_for_path` and `urzua_graph`, every one
advertised `readOnlyHint: true`. Their names carry a read word and no write word, so the harness's name-based reading of an MCP
call leaves them alone; that naming is load-bearing and a test pins it.

Read-only is a property of the code, proved by tests rather than asserted: no tool is named for a write and a call to an unknown
or mutating name is refused with the tree unchanged; the working tree and `.git` hash the same before and after every tool; a
source scan of the server's modules finds no write API (a planted file proves the scanner can fail); every stdout line parses as
a JSON-RPC frame; and the harness judges a read-named tool carrying a path as an allow and a write-named one as a write.

This ADR closes `MILE-47`. It builds in two changes: the server core with `urzua_check` and `urzua_list_records`, then the other
three tools, the read-only tests and the registration documentation. `MILE-29` is done when both have landed.

## Reversibility

High while unreleased and moderate after. The server is a subcommand and one crate that nothing else depends on, so removing it
is deleting them. Once users register it, the tool names, the input shapes and the pinned protocol revision are an interface
others write against, so changing them is a spec revision with a notice, not a refactor. Choosing an SDK later is possible and
costs a dependency review, because the tools sit behind the dispatch layer.

## Consequences

- The corpus's rejected and superseded decisions are reachable by an agent with no Bash, and the supersession chain with them.
- The engine owns a small protocol surface: `initialize`, `initialized`, `ping`, `tools/list`, `tools/call` and the standard
  error codes. A client that needs more (resources, prompts, sampling) is not served until a later decision.
- The guarantee that the server cannot write is the absence of write code, not the editor hook, which is advisory for agents
  (`SPEC-19`). The accepted-risk list of `ADR-89` is unchanged; the server adds no route to move a status or write a protected path.
- Each command's "build the report" step is split from its printing so the CLI and the server share it. That is a refactor of
  `urzua-cli` the build changes carry.
- A tool never answers "no findings" for a corpus it did not read: results carry the population or files examined (`ADR-55`).
- `MILE-29`'s Done means is met by the generic query, not a dedicated tool; the milestone is amended to say so when it closes.
- Mutating commands (`new`, `fix --apply`, `index --apply`, `migrate`, `reverify`, `init`, `harness install`) have no tool and
  will not get one under this decision. A write-capable server is a different decision.

## References

`MILE-29`, `MILE-47`, `RFC-27` (open question: engine or ecosystem), `ADR-1`, `ADR-5`, `ADR-13`, `ADR-23`, `ADR-26`, `ADR-46`,
`ADR-55`, `ADR-61`, `ADR-64`, `ADR-89`, `SPEC-19`, `SPEC-24`, `SPEC-26`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-13 | Filed as Accepted: accepted by the maintainer on 2026-11-13. **Why:** the maintainer's decision of 2026-11-13, taking the recommendation on each of the seven choices (engine transport, generic status filter, stdio only, `urzua mcp`, hand-written JSON-RPC crate, report JSON as the result, registration by snippet) and choosing an ADR with no separate RFC; it closes `MILE-47`. | **substantive** |
