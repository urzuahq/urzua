---
default: minor
closes: []
---

Add `urzua mcp`, a read-only Model Context Protocol server on stdio, so an agent can read the corpus without a shell. It
serves two tools, `urzua_check` (findings, scope and the rules run) and `urzua_list_records` (each record's id, type, path and
header fields, filtered by record type and by any status, so rejected and superseded decisions are one query). Every result is
the report the matching command prints, as structured content plus the same JSON as text. The server has no tool that writes.
