---
default: patch
closes: []
---

A tracked record file that is a symlink resolving outside the repository, or outside its record
type's directory, is no longer read by any command. `check`, `audit` and the MCP check tool report
it as a blocking `identity.record-symlink-escapes` finding naming the link, and every other command
that loads the corpus stops with exit 2 naming it. The rule is on for every configuration and
cannot be turned off, and no waiver excuses it. A link to another file in the same record directory
is still read.
