---
default: minor
closes: []
---

The agent harness checks the translator before it runs it. Every hook command `urzua harness install` and
`urzua harness update` write embeds the sha256 of `.claude/hooks/urzua-claude-code.py`, reads the file once
and runs it only if the hash matches; on a mismatch an edit is refused where `acceptance` or
`protected-paths` is on with `on_unjudged: deny`, and the other hooks say they did not run. An edit is
judged against the session's own repository and every governed repository that holds the file, inner or
outer, and a deny from any of them wins; a `.git` counts only when git accepts it, so an empty `.git` and a
configuration written above the project no longer replace the project's gates. An MCP tool named for reading
is judged when the call names an output or carries content, and an MCP path with surrounding whitespace, a
NUL, dot segments or an over-long run of them is read as the path it names.
