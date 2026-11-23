---
Stable-Id: 01M838RJMSKR7CE73FNPF96XJZ
Status: Fixed
Found-in: "CI run 45108278472 on an unrelated records-only pull request, where `without_the_engine_the_adapter_states_that_the_command_was_not_judged` panicked with `Os { code: 32, kind: BrokenPipe }` at the stdin write."
Realized-by: test:rust/crates/urzua-cli/tests/harness_post_command_integration.rs#a_hook_that_exits_without_reading_a_large_payload_is_not_a_failure@sha256:35200e3409b840062100f5fffc5f52c15dc38271f0e94820c838d86837eea8dc
---
# 216 — Test stdin helpers panic with a broken pipe when a hook exits without reading its input

## What was wrong

The test helpers that run an installed hook command wrote its payload to stdin and unwrapped the result. With no engine on `PATH` the hook's fallback exits without reading its input, so the write could fail with `EPIPE` (a race for a small payload, certain for one larger than the pipe buffer) and the test panicked. A hook is allowed to exit without reading its input. The helpers in `harness_post_command_integration.rs` and `block_on_integration.rs` now use `common::write_stdin`, which treats a broken pipe as "not read" and still panics on any other write error.

## Why nothing caught it

The fallback used to read the configuration and so lived long enough for the write to finish; once it was shortened the race opened, and no test wrote a payload the hook would not read.

## References

`a_hook_that_exits_without_reading_a_large_payload_is_not_a_failure` in `rust/crates/urzua-cli/tests/harness_post_command_integration.rs`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/tests/harness_post_command_integration.rs#a_hook_that_exits_without_reading_a_large_payload_is_not_a_failure@sha256:35200e3409b8 Reason: Pin the test that writes a 1 MiB payload to a hook that exits without reading it **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-23 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
