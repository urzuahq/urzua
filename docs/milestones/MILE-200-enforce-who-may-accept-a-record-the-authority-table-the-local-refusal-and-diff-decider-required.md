---
Stable-Id: 01M6NX36ZFJBB9DM1B17YJREVP
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-89
Depends-on: MILE-198
Blocked-on: null
---
# 200 — Enforce who may accept a record: the authority table, the local refusal and diff.decider-required

## What

Enforce who may accept a record, as `ADR-89` section 4 decides: the top-level `authority` configuration (named decider groups of GitHub logins, `evidence` kinds, `gates` by type and status and by path), the local refusal of an agent's move to a gated status or write to a protected path in `urzua harness run edit`, and the opt-in rule `diff.decider-required` with `urzua check --actors FILE`, fed by the gate action. It is blocked until the maintainer accepts `ADR-89`.

## Why

An agent edit that sets `Status: Accepted`, `Deciders` and `Accepted-on` on a Proposed ADR gives no finding at the editor, the commit or CI (`.urzua/config.yaml:88-91`, `:103`); today an agent is stopped from self-accepting only by its tool's permission classifier and by prose in `AGENTS.md`, neither of which urzua owns or tests. The same agent can edit the hook scripts, `.claude/settings.json` and `.urzua/config.yaml` that would stop it (`ADR-89`, Context). The maintainer decided on 2026-11-05 that this is enforced by the harness and its CI.

## Done means

- `authority` loads from the configuration. A group with no login, a login in no group, and a gate whose `type` or `to` the configuration does not declare are load errors naming the entry, each planted in a test.
- `urzua harness run edit` with actor `agent` denies an Edit, Write or MultiEdit that moves an `adr` or `rfc` to `Accepted` or `Rejected`, and one that writes a gated path; a human-actor request is unaffected; the deny cannot be overridden (the override variable is ignored for it and the output says so). Each case fails before the change.
- `diff.decider-required`, run with `--base` and an `--actors` file, reports an error for a gated move or gated path whose author or current approvers are not in the gate's group, and is clean for one that is. A gated move with no `--actors` file is reported as `info` on a local run and as an error under `require_evidence: true`; the rule reports not-applicable with no `--base`. A stale approval (before a later push) does not count (the gate action's fixture shows it).
- The gate action (`.github/actions/urzua-gate`) writes the `--actors` file from the GitHub API and passes it; its fixtures cover an approved, an unapproved and a self-authored acceptance.
- This repository declares `authority` for `adr` and `rfc` and for `.claude`, the hook files, `.git-hooks`, `.github/workflows` and `.urzua/config.yaml`; a planted acceptance by an agent session is denied locally, and `urzua check --base origin/main` reports 0 unwaived errors or warnings on this milestone's own PR. `urzua doctor` states that the required status check and branch protection are the host's and are not verified.
- `SPEC-2`, `SPEC-3` and `SPEC-19` document the rule, `--actors` and the table with revision rows; `make rule-table-check` passes; a minor changeset names `--actors`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
