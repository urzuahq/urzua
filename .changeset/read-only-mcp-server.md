---
default: minor
closes: []
---

Add `urzua mcp`, a read-only Model Context Protocol server on stdio, so an agent can read the corpus without a shell. It
serves five tools: `urzua_check` (findings, scope and the rules run), `urzua_list_records` (each record's id, type, path and
header fields, filtered by record type and by any status, so rejected and superseded decisions are one query),
`urzua_show_record` (one record in full with the relation edges that touch it), `urzua_get_findings_for_path` (which records
govern a path) and `urzua_get_graph` (every relation edge). Every result is the report the matching command prints, as structured
content plus the same JSON as text. The server has no tool that writes. Register it in your agent's MCP configuration; the
`docs/reference/mcp.md` page has the snippet.
