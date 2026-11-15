---
default: patch
closes: [BUG-195]
---

A tracked record file that is a symlink, or sits behind a symlinked directory, is no longer read by
any command. `check`, `audit` and the MCP check tool report it as a blocking
`identity.record-symlink-escapes` finding naming the link and where it resolves, and every other
command that loads the corpus stops with exit 2 naming it. No link is allowed, including one to
another file in the same record directory (`ADR-12.md` linked to `draft.txt`) or a chain of links.
The rule is on at `error` for every configuration, runs under every `--rules` selection, and no
waiver excuses it. A configuration may set another level under `rules:`, or `off`; with `off` a
record is read through the link and every `check` carries a warning notice saying so.
