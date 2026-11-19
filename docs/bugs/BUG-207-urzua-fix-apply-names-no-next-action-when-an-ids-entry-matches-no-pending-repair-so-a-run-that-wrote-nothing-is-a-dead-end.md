---
Stable-Id: 01M7SKPPHC3WJNHW1SXX7CGKTV
Status: Fixed
Found-in: an agent ran the command the editor hook named, urzua fix --apply --ids <record path> --actor agent, which wrote nothing and left next_action null
Realized-by: test:rust/crates/urzua-cli/tests/fix_exit_codes_integration.rs#a_failed_apply_exits_1_and_names_the_dry_run_that_lists_what_is_pending@sha256:ce20c7dcde9e076b3b2700a2be483cefe7cd8607467c9359eb8b637e51e19faa
---
# 207 — urzua fix --apply names no next action when an --ids entry matches no pending repair, so a run that wrote nothing is a dead end

## What was wrong

`urzua fix --apply --ids <entry>` with an entry that matched no pending repair reported `partial-failure` with a `failed` entry and `next_action: null`, so a caller that read the next action to find what to do had nothing to run and the run read as a dead end. The exit code was already right (1 for `partial-failure`, 2 for `not-run`, in every mode); the report gave no next step.

## Why nothing caught it

No test asserted the `next_action` of a failed `fix` run, only that it failed, and the exit codes of `fix` were documented in prose without a test per mode. The new tests pin the exit code of a failed apply and of a configuration that fails to load in each mode, beside the next action.

## References

`BUG-208` is the cause of the entry not matching in the case that surfaced this. Fixed in `rust/crates/urzua-cli/src/commands/fix.rs`: a failure naming an `--ids` entry carries `urzua fix` (`urzua fix --rebaseline` for a re-pin) as its next action. `SPEC-8` and `docs/reference/commands.md` state it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
