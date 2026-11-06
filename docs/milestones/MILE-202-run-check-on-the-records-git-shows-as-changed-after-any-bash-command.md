---
Stable-Id: 01M6NX38DH7TTY0H3YEJQ0XQ58
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-89, ADR-78
Depends-on: MILE-198
Blocked-on: null
Realized-by: code:rust/crates/urzua-cli/src/harness/post_command.rs, code:rust/crates/urzua-cli/src/harness/run.rs, code:rust/crates/urzua-cli/src/harness/mod.rs, code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-cli/src/discovery.rs, code:rust/crates/urzua-cli/bundle/claude-code/files/.claude/hooks/urzua-claude-code.py, code:rust/crates/urzua-cli/bundle/claude-code/settings.fragment.json, code:rust/crates/urzua-cli/bundle/claude-code/adapter.json, code:.urzua/config.yaml, code:.claude/settings.json, test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs, spec:docs/specs/SPEC-24-the-harness-request-verdict-and-adapter-layout.md, spec:docs/specs/SPEC-19-repository-agent-guardrails.md
---
# 202 — Run check on the records git shows as changed after any Bash command

## What

Build the post-command check specified by the 2026-11-05 amendment to `ADR-78`: after any Bash command, `urzua harness run post-command` takes the changed paths from `git status --porcelain -z` (untracked included), keeps the governed records, runs `urzua check PATHS` on all of them in one call (a named untracked file is examined, `rust/crates/urzua-cli/src/commands/check.rs:446-470`, `rust/crates/urzua-cli/src/discovery.rs:321`, `BUG-24`), and reports the unwaived findings to the agent, without reading the command. No `--stdin-path` relay is built. The Claude Code adapter wires it to the Bash tool.

## Why

The editor hook's matcher does not cover Bash (`.claude/settings.json:5`), so a record written by `sed -i`, a heredoc or an interpreter one-liner is never judged at the editor. `check --base` cannot stand in, because its changed set is a tracked-files diff (`rust/crates/urzua-io/src/lib.rs:179-189`) that misses a record a command creates, and `check --base` refuses a named untracked file (`check.rs:451-456`); `check PATHS` without `--base` examines it, which is why no relay is needed. Pre-commit and CI are the hard gate (`ADR-71`); the agent should hear about a bad record when it writes it, not at the commit.

## Done means

- After a Bash command that edits a governed record with `sed -i`, one that creates a new record with a heredoc (untracked, and the working tree and index are unchanged afterwards), and one that rewrites one with an interpreter, the agent receives the unwaived findings with file, rule and message; each case fails before the change. A command that changes no governed record produces no report and runs no `check`.
- A record already failing before the command is listed as pre-existing by comparison with `HEAD`; an unchanged record's findings are not re-reported on the next command (keyed by content hash); a deleted record is not judged.
- With `git` absent or `check` unable to run, the report says the command's effect on records was NOT judged (a test for each); the check never blocks and never reports success it did not earn.
- A Bash command that moves a record to a status a gate names, or changes a gated path, is reported as an error-level item of its own, with the acceptance finding of `MILE-200`, when that guardrail is enabled; the report says it was not prevented. This is soft-dependent on `MILE-200` (the finding), and the rest of this milestone does not wait for it.
- This repository enables `post-command`, and a planted Bash write of a bad header in a session is reported; `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings.
- `SPEC-2` or the harness spec documents the check with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | Corrected after review: untracked records are examined by `check PATHS` (`BUG-24`), so the `--stdin-path` relay is dropped and the What and Why no longer say otherwise; a Bash-made gated move is reported as an error of its own. **Why:** Independent review of pull request 355 (the untracked-file claim was false). Still `Planned`. | **substantive** |
> | 2026-11-06 | Built and marked Done. `urzua harness run post-command` takes the changed paths from `git status --porcelain=v1 -z --no-renames --untracked-files=all`, keeps the governed records, drops those whose content hash was already reported (`.urzua/cache/post-command.json`) and runs `check PATHS` once, in process, over the rest; it lists a finding the record already had at `HEAD` as pre-existing, does not judge a deleted record, says NOT judged for a missing `git`, a tree git does not know, a configuration that does not load, an unreadable request and a `check` that cannot run, always exits 0, and is on only when `harness.guardrails.post-command` is named. The Claude Code adapter wires it as a `PostToolUse` hook on `Bash`; this repository enables it and has the bundle installed through `urzua harness install`. Tests plant the `sed -i`, heredoc and interpreter writers and run the installed hook command. On this repository `git status` costs about 6 ms and the whole hook about 55 ms when nothing new needs checking; a Bash call that newly changes a governed record pays one `check PATHS`, about 1.2 s here, the engine's own corpus-wide cost. The Done means item for a gated move is satisfied through the finding that `MILE-200` adds: it rides `check PATHS` and is reported with its severity and the not-prevented statement, and no test pins it until `MILE-200` lands. **Why:** `ADR-78` amendment (post-Bash check), `ADR-89`. | **substantive** |
