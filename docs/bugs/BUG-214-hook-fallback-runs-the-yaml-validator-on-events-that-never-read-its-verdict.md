---
Stable-Id: 01M7XYZXYFDRFWMS8K2T2FJK1V
Status: Fixed
Found-in: a maintainer's macOS session on 2026-11-21, where, with a urzua too old to have `harness hook`, the session-start fallback took about 4.5 minutes on a 257-line configuration and the agent host timed out the turn. Read from `command.rs`; not slow on Linux (0.05-0.09 s).
Realized-by: test:rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_only_the_edit_fallback_reads_the_configuration@sha256:71c7de32d6fbbda2024f2fb7e21db22e00bfdbbd99ebb577271373c3560017a0
---
# 214 — Hook fallback runs the YAML validator on events that never read its verdict

## What was wrong

`FIND_CONFIG` in `rust/crates/urzua-cli/src/harness/hook/claude_code/command.rs` both found `.urzua/config.yaml` (`found`) and, for each one found, ran the NUL check and the `CERTAIN_ALLOW` awk validator (`bad`). Every event's fallback began with it, but only the edit fallback reads `bad`; the post-command and session-start fallbacks print their message whatever it is. They ran the validator for an answer they threw away, and on a platform where awk is slow on that program the session-start hook outlasted the host's timeout.

## Why nothing caught it

The fallback tests assert each event's output, never what the command runs to reach it, so a validator run whose result is unused is invisible. Linux awks run it in under 0.1 s, so no timing check reached the failure.

## References

Fixed by walking with `found` only for the post-command and session-start fallbacks (`READ_CONFIG` is spliced in for the edit fallback alone). Test: `without_urzua_only_the_edit_fallback_reads_the_configuration` runs each fallback with a recording `awk` on PATH and asserts only the edit one calls it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-21 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/tests/harness_fallback_integration.rs#without_urzua_only_the_edit_fallback_reads_the_configuration@sha256:71c7de32d6fb Reason: pin the new fallback test as evidence **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-21 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
