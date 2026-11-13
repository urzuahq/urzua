---
default: minor
closes: []
---

Add three tools to `urzua mcp`: `urzua_show_record` (one record in full, with the relation edges that touch it),
`urzua_get_findings_for_path` (which records govern a path) and `urzua_get_graph` (every relation edge). Each returns the report
the matching command prints, and none writes. The server's registration is documented in `docs/reference/mcp.md`.
