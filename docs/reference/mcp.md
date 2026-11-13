# The MCP server

`urzua mcp` is a read-only [Model Context Protocol](https://modelcontextprotocol.io) server. It lets an
AI agent read your records, findings and relations without a shell. It is a transport, not a new
capability: every tool returns the report that the matching command already prints. It has no tool that
writes, and nothing in it can change your repository.

## Register it

Registration is your act, in a file you own. `urzua harness install` does not write MCP configuration.
For a client that reads a `.mcp.json` in the project root:

```json
{ "mcpServers": { "urzua": { "command": "urzua", "args": ["mcp"] } } }
```

Or, with Claude Code, `claude mcp add urzua -- urzua mcp`. Keep the server name `urzua`: the editor
hook of [the agent harness](agent-harness.md) reads an MCP call by its tool name, `mcp__urzua__<tool>`,
and every tool below has a name it leaves alone.

The server serves the repository that holds the directory it was started in. When your client starts
servers somewhere else, pass the repository: `"args": ["mcp", "--root", "/path/to/repo"]`. It exits 2,
with a JSON could-not-run on stderr and nothing on stdout, when it cannot read `.urzua/config.yaml`.
`--as-of` is refused.

## The tools

Every tool is advertised read-only and takes only the arguments listed; an argument it does not
declare is refused, never ignored. A result carries the report as `structuredContent` and the same JSON
as one text block. A failure that `urzua` would report as could-not-run (an unreadable corpus, an unknown
id, an unknown rule) is a result with `isError: true` and `"status": "not-run"`.

| Tool | Arguments | Returns |
|---|---|---|
| `urzua_check` | `paths`, `base`, `rules` (all optional) | The `check` report: findings, scope and rules run. `paths` stay inside the repository. See [`check`](check.md). |
| `urzua_list_records` | `type`, `status` (a list), both optional | Each matching record's id, type, path and header fields, without its text. A filter that matches nothing is a notice. |
| `urzua_show_record` | `id` (required, for example `ADR-23`) | One record: header, sections and complete text, and the relation edges that touch it, supersession included. An unknown id is an error result. |
| `urzua_get_findings_for_path` | `path` (required) | Which records govern a file, directory or glob: the report of [`urzua explain`](commands.md#explain). |
| `urzua_get_graph` | none | Every relation edge between records: the report of [`urzua graph`](commands.md#graph). |

### What was already rejected or superseded

Ask for the decisions your corpus has turned down, then read why:

1. `urzua_list_records` with `{"type": "adr", "status": ["Rejected", "Superseded"]}`. The status names
   are your own: the server knows none, so use the ones your config declares.
2. `urzua_show_record` with one of the ids. The reasons are the prose of its sections, returned as
   written, and a superseded record's `edges` name the record that replaced it.

## What it does not do

- It writes nothing: no record, no working file, no git object or ref. There is no tool for `new`,
  `fix --apply`, `index --apply`, `migrate`, `reverify`, `waive --apply` or the harness. (Reading a
  repository can make git refresh its own index; that is git's cache, not your files.)
- It listens on no port: stdin and stdout only. A request line is read up to 1 MiB.
- It serves tools only: no resources, prompts or sampling.

The design is `ADR-99` and the contract is `SPEC-26`.
