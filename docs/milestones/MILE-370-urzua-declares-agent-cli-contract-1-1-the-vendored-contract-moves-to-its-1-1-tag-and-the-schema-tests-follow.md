---
Stable-Id: 01M85PWE8NAM86Y5ME164DMBN4
Status: Planned
Phase: '1'
Track: integrations
Depends-on: MILE-359, MILE-365, MILE-366, MILE-367, MILE-368, MILE-369
Blocked-on: —
---
# 370 — urzua declares agent CLI contract 1.1: the vendored contract moves to its 1.1 tag and the schema tests follow

## What

urzua prints `contract_version` `"1.1"` in `describe`: the vendored contract fixtures move to the contract's final 1.1 tag (`rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/VERSION` and the schemas and `SHA256SUMS`), `agent_cli_contract_integration.rs` validates against them, and a changeset says what an adopter gets.

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (this milestone). The contract's 1.1 tag waits for both tools to pass the runner against a release candidate (RFC-4 section 10), which `MILE-369` vendors, so this is last: it moves the vendored copy from that candidate to the final tag. It is the only milestone that changes the claim urzua makes; the others change behaviour. The vendored schemas are re-pinned only after reading the diff of the pinned test and fixture (`AGENTS.md`, "Re-pinning evidence"), and each re-pin is listed in the PR body.

## Done means

- `VERSION` names the 1.1 tag and its commit, the vendored schemas and `SHA256SUMS` are byte-identical to that tag, and `describe` prints `contract_version` `"1.1"`.
- `agent_cli_contract_integration.rs` validates `check`'s report, a `not-run` report, a report with a placeholder and `describe` against the 1.1 schemas, and the tests fail against a report missing `next_action_argv`.
- `MILE-359`, `MILE-365`, `MILE-366`, `MILE-367`, `MILE-368` and `MILE-369` are `Done` with evidence in the same or earlier PRs (a milestone is not set `Done` here by assertion).
- A changeset (minor) in `.changeset/` carries `closes:` for the records the PR closes and says in a sentence each that a hook now blocks without the binary, `hooks.on_not_run` is gone, `error_kind` and the argument forms exist, and `describe` declares inputs.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
