---
Stable-Id: 01M85WEVHZQMRTYR6K4Q0JJKTH
Status: Fixed
Found-in: "Review of the pull request for `MILE-367`: the serialise-failure branch of `emit_as` in `rust/crates/urzua-cli/src/main.rs` prints a `CouldNotRun` that skips `with_next_action`."
Realized-by: test:rust/crates/urzua-cli/src/main.rs#the_serialise_failure_envelope_carries_the_next_action_keys@sha256:096cf6abb31649c80fef8f7db0f91d34ed658b961b335d6edd9d53d019620aae
---
# 225 — The fallback envelope for a report that cannot be serialised has none of the next_action keys

## What was wrong

When a report cannot be written as a JSON object, `urzua` prints a could-not-run envelope with only `status` and `error`. Every other envelope carries `next_action`, `next_action_argv`, `next_action_reason` and `next_action_inputs`, so a caller reading those keys finds them absent on this one path.

## Why nothing caught it

No input makes a report fail to serialise, so no test reached the branch, and it built its envelope without the wrapper every other print goes through.

## References

The branch now builds its envelope through `with_next_action`. The test calls the helper with a real serde error; before the fix it failed with `missing next_action`, after it passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/src/main.rs#the_serialise_failure_envelope_carries_the_next_action_keys@sha256:096cf6abb316 Reason: Pin the unit test that asserts the serialise-failure envelope carries the next_action keys **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
