---
default: patch
closes: []
---

The agent harness no longer lets an edit through that it could not judge. A request the engine cannot read (a tool input with a lone surrogate escape, for one), or whose working directory cannot be entered, is refused for an agent where `acceptance` or `protected-paths` is set to `on_unjudged: deny`, and a working directory that cannot be entered is replaced by the session's project directory, which the Claude Code translator also runs the engine in. An MCP call that carries two different strings under content keys is treated as an edit the translator cannot place, instead of judging one of them. The engine judges at most 512 paths of a request and refuses the rest under deny, and asks git about each directory once. `urzua harness status` reports a `disableAllHooks` kill switch, and compares the installed bundle's version with the binary's as numbers (`0.10.0` is newer than `0.9.0`).
