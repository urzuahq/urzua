---
Stable-Id: 01M6NX38DH7TTY0H3YEJQ0XQ58
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-89, ADR-78
Depends-on: MILE-198
Blocked-on: null
---
# 202 — Run check on the records git shows as changed after any Bash command

## What

Build the post-command check specified by the 2026-11-05 amendment to `ADR-78`: after any Bash command, `urzua harness run post-command` takes the changed paths from `git status --porcelain -z` (untracked included), keeps the governed records, runs `urzua check` on the tracked ones and `urzua check --stdin-path` on each untracked one (discovery reads only tracked and staged files, `rust/crates/urzua-io/src/lib.rs:56,75`), and reports the unwaived findings to the agent, without reading the command. The Claude Code adapter wires it to the Bash tool. It is blocked until the maintainer accepts the amendment.

## Why

The editor hook's matcher does not cover Bash (`.claude/settings.json:5`), so a record written by `sed -i`, a heredoc or an interpreter one-liner is never judged at the editor. `check --base` cannot stand in, because its changed set is a tracked-files diff (`rust/crates/urzua-io/src/lib.rs:179-189`) that misses a record a command creates, and `check PATHS` does not examine an untracked file at all. Pre-commit and CI are the hard gate (`ADR-71`); the agent should hear about a bad record when it writes it, not at the commit.

## Done means

- After a Bash command that edits a governed record with `sed -i`, one that creates a new record with a heredoc (untracked, and the working tree and index are unchanged afterwards), and one that rewrites one with an interpreter, the agent receives the unwaived findings with file, rule and message; each case fails before the change. A command that changes no governed record produces no report and runs no `check`.
- A record already failing before the command is listed as pre-existing by comparison with `HEAD`; an unchanged record's findings are not re-reported on the next command (keyed by content hash); a deleted record is not judged.
- With `git` absent or `check` unable to run, the report says the command's effect on records was NOT judged (a test for each); the check never blocks and never reports success it did not earn.
- A Bash command that moves a record to a status a gate names is reported with the acceptance finding of `MILE-200` when that guardrail is enabled.
- This repository enables `post-command`, and a planted Bash write of a bad header in a session is reported; `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings.
- `SPEC-2` or the harness spec documents the check with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
