---
Stable-Id: 01M86B17HAP1JCCTJMCPQ97K33
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: —
---
# 372 — `urzua harness run` prints no `next_action` or `next_action_reason`

## What

`urzua harness run` prints its verdict with no `next_action`, `next_action_argv`, `next_action_reason` or `next_action_inputs` keys, as `urzua harness hook` and `urzua harness record` already print none of their own.

## Why

The maintainer decided this on 2026-11-24, in the agent session that built the pull request that added `urzua describe` (#647); the decision is not recorded in the repository or the pull request, so it cannot be checked there. The agent asked: "#647 is green and reviewed. Its last open point: `urzua harness run` prints its verdict as a report, so under your rule ("next_action applies to commands that print a report") it keeps `next_action`, always null. My earlier question had wrongly said it prints no report. OK to keep it that way and merge #647?" The options were "Yes, merge" (keep the keys) and "No" (remove the keys from `harness run`; change SPEC-2's wording and file a follow-up before merging). The maintainer answered: "No".

Until this ships, the verdict still prints the keys, always `null`. A search of the plugin (`plugins/`), `.claude/`, `.git-hooks/`, `.github/` and the reference docs found nothing that reads `next_action` from `harness run` output; `docs/reference/commands.md` says every report carries the keys, which this changes for the verdict.

## Done means

- The run verdict no longer goes through the key-adding path: `harness/run.rs` (`run_edit`, `run_session_start`, and the verdict emitters near them) and `run/question.rs` (its `emit` calls) print the verdict without it.
- A test asserts the four `next_action` keys are absent on each `urzua harness run` event.
- `SPEC-2` section Next action lists `urzua harness run` among the commands without the keys, and `docs/reference/commands.md` says so.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
