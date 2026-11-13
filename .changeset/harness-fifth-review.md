---
default: patch
closes: []
---

The agent harness closes the gaps the fifth review found. `urzua harness run post-command` now
reports a move to a gated status even when the same command removed or narrowed the status gate or
switched `acceptance` off, and reports that loss (`session-start` does too); a new session forgets
what an earlier one was told, so a change made again is reported again. The Claude Code translator
reads a path used as a dictionary key and a `file:` URL in any case, no longer drops strings beyond
the 64th (a call with more than 512 distinct strings or files, or more than 8 files checked as records, is
refused as unjudged under `on_unjudged: deny`), judges the acceptance and protected-path gates of
every file in an MCP call in one engine process, however many files, before it checks any record and
stops at the first block (a call with 2200 files no longer outlasts the hook timeout), and the edit
hook sets a 120 second timeout. The neutral request gains `gates_only` and `contents` and the verdict `record` and `records`. A
status gate whose decider group is changed is reported as the move and as a change to the
configuration, not as a shrunk gate.
