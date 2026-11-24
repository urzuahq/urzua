---
Stable-Id: 01M85PW9Q2XAPTTHHH0CJRQVTD
Status: Planned
Phase: '1'
Track: integrations
Depends-on: MILE-359, MILE-365
Blocked-on: —
---
# 366 — No command prompts or depends on a terminal, and stderr is empty unless the structured log is asked for

## What

No command reads from a terminal, waits for a person or changes its behaviour or output by whether a stream is a terminal: `init`'s menu (`can_prompt`, `prompt_selection` and `run_interactive` in `init_greenfield.rs`) is removed and the `--types` path is the only one. A command that prints a report writes nothing to stderr unless the caller turns on the structured log declared in `describe` (`log`), which writes one JSON object per line with `time`, `level` and `message`.

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). The audit of `eprintln!` and stderr in `rust/crates/*/src` finds, outside the commands that speak a host's protocol (`mcp`, `harness hook`, `hooks run`, whose streams the contract leaves to the host): `waive.rs` (three notices printed to stderr beside the same notices in the report), `new.rs` (three, the pull-ref fetch notice among them), `harness/record.rs` (12, the review text and the lines an agent is told to list in a pull request body, which `AGENTS.md` relies on), `harness/post_command.rs` (none: its `stderr` matches are `.discard_stderr()`, and it is a protocol hook, so it is out of this milestone) and `init_greenfield.rs` (the menu). The contract's own rule is that a warning about the tool's environment is a `notices` entry, which `waive` and `new` already also carry, so their stderr copies are the removable ones; `harness record` and `post_command` print for a person at a terminal and need a decision about where that text goes (a report key or a file the report names), which this milestone makes and records. The log is optional in the contract (`log` null is valid), so the narrowest conforming version declares `log: null` and only silences stderr; a log is built only if a real consumer asks (`AGENTS.md`, speculative capability).

## Done means

- A test runs `urzua init` with standard input a terminal-less pipe and with a pseudo-terminal and gets the same report both times, and `can_prompt` and `prompt_selection` no longer exist.
- A test runs every command `describe` marks `safe`, with the log off, and asserts stderr is empty; it fails against the current `waive` and `new` notices.
- The `eprintln!` sites listed in Why are each removed, moved into `notices` or a named report key, or listed in the revision log as protocol streams with the contract's `protocol` word; a grep of `rust/crates/*/src` for `eprintln!` finds only those.
- A panic prints nothing to stderr (the runtime's panic message is silenced) and the report of `MILE-365` on stdout, shown by a test.
- `harness record`'s review text and its pull-request-body lines reach the agent through the report or a named file, and `AGENTS.md`'s instruction to list every `Pinned` and `Re-verified` line follows (any edit under `.claude`, a protected path, is the maintainer's).
- `harness record` has no exemption from the stderr rule (the contract's RFC-4 section 2): a command printing the contract's report on stdout writes nothing on stderr, and the review text and the `Pinned` and `Re-verified` lines become keys in its JSON report. Those keys are urzua's own extension keys; that they do not collide with the base keys of the contract's report is urzua's own constraint, not a rule RFC-4 states. A command is either exempt because it speaks a host's protocol (RFC-4 section 5) or under section 2, never both. A test runs `harness record` and finds stderr empty and those lines in the report, and a second finds no extension key equal to a base key.
- Either `describe`'s `log` is null and no log code exists, or the log's input named in `describe` turns on stderr lines that validate against the contract's log-line schema with the report and exit code unchanged, shown by a test.
- No command prints colour, a pager or a progress display, and a test that runs the commands with a terminal-like environment (`TERM` set, `NO_COLOR` unset) and without it compares the bytes of stdout and finds them equal.
- A warning from the runtime or a dependency is silenced or captured by urzua, shown by a test that provokes one (a deprecated-alias notice is already a `notices` entry) and finds nothing on stderr.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
> | 2026-11-24 | Added a Done-means item (additive): RFC-4 section 2 reads as giving `harness record` no exemption from the stderr rule (RFC-4 does not name the command; the contract's thread confirmed this reading); its review and `Pinned`/`Re-verified` lines become extension keys in the JSON report. The Why's remark that `harness record` needs a decision about where its text goes is settled by it. | **substantive** |
