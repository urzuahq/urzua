---
Stable-Id: 01M8394E5MQBYX04A530GYM063
Status: InProgress
Phase: '1'
Track: integrations
Blocked-on:
---
# 359 — Reports carry a schema version and urzua describe states the commands, scopes and metrics a gate can call

## What

Every JSON report carries a `schema_version`, and a new `urzua describe` command prints what a
consuming gate needs to call urzua without an adapter: `name`, `version`, `contract_version`,
`system`, and `commands[]`, each with `name`, `profile` (`base`, `gate` or `exempt`), `safe`,
`scopes` and `metrics`. Both are additions: no existing report key is renamed or removed.

## Why

`MILE-358` names a `describe` answer as the other half of a gate reading urzua natively, and left
its shape unsettled. The field list above is the proposal; the rule it follows is settled, that
urzua's existing shape is the contract and urzua conforms by addition only: the maintainer accepted that plan on 2026-11-23, after a review
found that changing the report's shape would break consumers silently (`hooks.rs` reads
`next_action` with `as_str()`), would contradict `RFC-3`'s non-goal of a shared status enum, and
would reintroduce the shared envelope type `ADR-46` rejected. Every reporting command already
prints `status`, `next_action`, `next_action_reason` and `next_action_inputs` (checked 2026-11-23
against `check`, `explain`, `graph`, `board`, `roadmap`, `audit`, `export`, `doctor`, `rules`,
`index`, `refs`, `fix`, `reverify`, `waive`, `hooks`, `harness status`, `harness adapters`,
`harness limits`, `harness record`, `harness uninstall --dry-run`, `migrate`, `import`, `init` and
`new`), so the base layer needs nothing changed. `mcp` speaks JSON-RPC and `harness hook` prints
the agent's hook format; `harness run` prints a verdict with no `status`. All three would be
`exempt`, a profile for a command a gate does not call as a check.

## Done means

- The shape of `describe` and of `schema_version` (its value, and which reports carry it) is recorded in an amendment to `ADR-46` or a new ADR citing this milestone.
- `urzua describe` prints the fields above, listing every subcommand with its `profile`, and a test shows that a subcommand added without a `describe` entry fails.
- Every report that carries `status` also carries `schema_version`, shown by a test that runs each reporting command.
- A test validates `urzua check`'s report against the published gate-report JSON Schema, vendored as a fixture. — evidence: test:rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#check_reports_validate_against_the_gate_report_schema@sha256:8d1a8809e874ddf1fd50fcfceb2ec887c9d6a1b4058bd01e4dd0947b49f64ba6, code:rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/VERSION@sha256:b1b241db78ef2678f85eacd0f820c89e382a09ed7b6144a899189a0af16b54b6
- `SPEC-2` and the reference pages describe `describe` and `schema_version`; each spec carries a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Filed as planned work: nothing built, no evidence yet. The addition-only rule is the maintainer's plan, accepted on 2026-11-23 and relayed from the gate tool's design thread: "the shared contract is urzua's existing shape, and urzua conforms by addition only". | **structural** |
> | 2026-11-23 | The maintainer replied on 2026-11-23 "Yes, we need to get product surface unified", approving the published agent CLI contract's schema fixes; the org coordination thread read that reply as also settling two points it had recommended, and told the maintainer so they can reverse it: the contract owns the `describe` field list, so the ADR in Done means item 1 would record adopting that list; and the schema in item 4 is a copy vendored from a contract release tag, with a version file and a checksum compared in CI (the thread's proposal), so that item waits for the first tag. The items themselves are unchanged. | **structural** |
> | 2026-11-23 | Evidence added for the fourth Done means item only: a test validates `check`'s report, and a `not-run` report, against the vendored gate-report schema (contract release `v1.0.0-rc.1`); a second test validates a placeholder-bearing report against the base schema. The other items are unchanged and have no evidence. A CI comparison of the vendored checksums against the release tag is not part of this change (it lives in a protected workflow path). | **structural** |
> | 2026-11-23 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 1 file-level, 1 scoped. Evidence: rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#check_reports_validate_against_the_gate_report_schema@sha256:7536867ba405, rust/crates/urzua-cli/tests/fixtures/agent-cli-contract/VERSION@sha256:b1b241db78ef Reason: Pin the evidence this change delivers: the Examined field, the check run path and the tests that show it **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-23 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#check_reports_validate_against_the_gate_report_schema` (lines 129-147) 7536867ba405 -> 8d1a8809e874. Reason: rustfmt reformatted the new contract test file after the first pin; no behaviour changed **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-23 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (1 of 5 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
