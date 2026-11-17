---
default: minor
closes: []
---

BREAKING: the agent harness runs from the binary. `urzua harness hook <adapter> <event>` does what the installed translator script did, and `urzua harness record` what the drafting helper did; `urzua harness install` writes no executable file into the repository and no hook entry carries a file hash. Reformatting what `install` wrote (a generated command, the instructions block, the lock, the settings file) is no longer drift. A Claude Code plugin in `plugins/claude-code` registers the same hooks and the lifecycle commands, and `urzua harness status` reports which registration is in effect. An allowed edit no longer says anything about an editor hook the configuration turned off.

There is no migration command. In a repository installed by 0.4.8 or earlier, remove by hand:

- `.claude/hooks/urzua-claude-code.py` and `.claude/hooks/urzua-record.sh` (`git rm` both);
- the three hook entries in `.claude/settings.json` whose command mentions `.claude/hooks/urzua-claude-code.py` (PreToolUse on `Write|Edit|MultiEdit|NotebookEdit|mcp__.*`, PostToolUse on `Bash`, SessionStart);

then run `urzua harness update` (`urzua harness install` where there is no `.urzua/harness.lock`) and check that `urzua harness status` prints `"status": "ok"`. `urzua harness status`, `urzua doctor` and the session-start check name each of these while it is still there. Where the script, the helper and the entries still match the lock, `urzua harness update` alone makes the same change.
