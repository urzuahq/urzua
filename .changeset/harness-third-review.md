---
default: patch
closes: []
---

The agent harness no longer lets an edit through when its hook command runs from a directory other than
the project root: the settings command finds the project from `CLAUDE_PROJECT_DIR`, else from the
outermost `.urzua/config.yaml` above the current directory, and when it cannot run the translator it
refuses the edit where `acceptance` or `protected-paths` is on with `on_unjudged: deny`. An edit is judged
by the outermost governed repository, so a nested git repository no longer takes its files out of the
gates, and a write through a symbolic link whose target does not exist yet is judged as the file it would
create. The edit tools are refused for the files the harness keeps (`.urzua/cache`, `.urzua/harness.lock`,
the override log, `.git/hooks`, `.git/config`). MCP tool calls are read for every path-like string at any
depth (`from`, `source`, `uri`, `files[]`). The session-start check compares the installed files and hook
entries with the lock and reports `disableAllHooks`. The post-command check lists a record hidden with
`git update-index --assume-unchanged` or `--skip-worktree`. A settings file nested more than 128 levels is
refused instead of overflowing the stack.
