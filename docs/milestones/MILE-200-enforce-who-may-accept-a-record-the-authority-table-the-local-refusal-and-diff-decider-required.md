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

Enforce who may accept a record, as `ADR-89` section 4 decides: the top-level `authority` configuration (named decider groups with `logins`, `evidence` kinds defaulting to `pr-author`, `gates` by type and status and by path), the local refusal of an agent's Write, Edit or MultiEdit that moves a record to a gated status or writes a protected path in `urzua harness run edit`, and the opt-in rule `diff.decider-required` with `urzua check --actors FILE`, fed by the gate action. It claims only what `ADR-89` section 4 states: Bash writes are reported, not blocked, and with a single maintainer CI proves authorship only. It is blocked until the maintainer accepts `ADR-89`.

## Why

An agent edit that sets `Status: Accepted`, `Deciders` and `Accepted-on` on a Proposed ADR gives no finding at the editor, the commit or CI (`.urzua/config.yaml:88-91`, `:103`); today an agent is stopped from self-accepting only by its tool's permission classifier and by prose in `AGENTS.md`, neither of which urzua owns or tests. The same agent can edit the hook scripts, `.claude/settings.json` and `.urzua/config.yaml` that would stop it (`ADR-89`, Context). The maintainer decided on 2026-11-05 that this is enforced by the harness and its CI, and also that agents acting under the maintainer's token are a documented, accepted risk (`ADR-89`, "Accepted risk").

## Done means

- `authority` loads from the configuration; `acceptance` or `protected-paths` enabled with no `authority` key is a load error. `evidence` defaults to `pr-author`, and a test shows a single-maintainer pull request authored by the decider passes. A group with no login, a login in no group, and a gate whose `type` or `to` the configuration does not declare are load errors naming the entry, each planted in a test.
- `urzua harness run edit` with actor `agent` denies an Edit, Write or MultiEdit that moves an `adr` or `rfc` to `Accepted` or `Rejected`, and one that writes a gated path; a human-actor request is unaffected; the deny cannot be overridden (the override variable is ignored for it and the output says so). Each case fails before the change.
- `diff.decider-required`, run with `--base` and an `--actors` file, reports an error for a gated move or gated path whose author or current approvers are not in the gate's group, and is clean for one that is. A gated move with no `--actors` file is reported as `info` on a local run and as an error under `require_evidence: true`; the rule reports not-applicable with no `--base`. A stale approval (before a later push) does not count (the gate action's fixture shows it). The rule's documentation states that with a single maintainer it cannot tell an agent from the maintainer.
- The gate action (`.github/actions/urzua-gate`) writes the `--actors` file from the GitHub API and passes it; its fixtures cover an approved, an unapproved and a self-authored acceptance.
- `urzua harness run post-command` reports a Bash-made gated move or protected-path change as an error-level item of its own (that command is built in `MILE-202`; its case is tested there and pinned here by a fixture).
- `SPEC-19` and the agent-instructions block carry an "Accepted risk" statement listing the Bash, shell-edited guardrail, self-modifying workflow and shared-token routes, with the date the maintainer accepted it.
- This repository declares `authority` for `adr` and `rfc` and for `.claude`, the hook files, `.git-hooks`, `.github/workflows` and `.urzua/config.yaml`; a planted acceptance by an agent session is denied locally, and `urzua check --base origin/main` reports 0 unwaived errors or warnings on this milestone's own PR. `urzua doctor` states that the required status check and branch protection are the host's and are not verified, and names a distinct agent identity plus `CODEOWNERS` as a future option and not as configured.
- `SPEC-2`, `SPEC-3` and `SPEC-19` document the rule, `--actors` and the table with revision rows; `make rule-table-check` passes; a minor changeset names `--actors`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | Rewritten after review to claim only what holds: the local refusal covers the edit tools, Bash is reported, and in CI the evidence is authorship, so a single maintainer cannot be told from an agent. `evidence` defaults to `pr-author` (never `pr-approver`, which locks out a single maintainer), decider groups use `logins`, an enabled guardrail with no `authority` key is a load error, and the accepted risk is documented. **Why:** Independent review of pull request 355 and the maintainer decision of 2026-11-05 to document the identity limit and accept the risk. Still `Planned`. | **substantive** |
