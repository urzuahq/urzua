---
Stable-Id: 01M82KGY405G1ZRC22FNJEJ1Q6
Status: InProgress
Phase: '1'
Track: integrations
Blocked-on:
---
# 358 — The check report carries what it examined as one count per unit, so a consuming gate reads it without an adapter

## What

`urzua check` reports what it examined as an object with one count per unit, `examined: {files: N}`,
using the unit names a consuming gate shares (`files`, `tests`, `packages`), so a tool that gates
on urzua's report reads it directly instead of through an adapter that renames `files_examined`.
Whether `files_examined` stays as an alias, and for how long, is decided as part of this work.

## Why

A gate tool that runs `urzua check` as a check without an adapter (a "native" tool) settled on
`examined` as an object of per-unit counts in its own design review on 2026-11-22. urzua's report
has a top-level `files_examined` integer (`ADR-46`, the shared output contract; `SPEC-2`, `SPEC-26`), so today the gate needs a
mapping for the one field that proves a check did not run on nothing. Renaming a report key is a
change to the output contract every consumer reads, so the shape and any alias are a recorded
decision, not a silent edit. A `describe` answer (name, scopes accepted, metrics reported) is the
other half of "native"; its shape is not settled yet and is not part of this milestone.

## Done means

- The decision on the key's shape and on keeping `files_examined` as an alias is recorded in an ADR, or in an amendment to `ADR-46`, which governs the report's shape, citing this milestone.
- `urzua check` reports `examined` with a `files` count equal to `files_examined` over the same corpus, shown by a test. — evidence: code:rust/crates/urzua-core/src/report.rs#Examined@sha256:2167ce1d8e62a3eba33c335b111ae5741481e6940a1ecf5da90ee67ea679bd30, code:rust/crates/urzua-cli/src/commands/check.rs#run_opened@sha256:725ff7ebf1909dc9c8494ca65c520a0d0f5f943dd7830ed2683760cb2386fe17, test:rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#check_reports_examined_files_equal_to_files_examined@sha256:a4df9f3e649772fcf20bb92a0a5f5ec969d52de2cacf68f127b8002482415516
- `SPEC-2`, `SPEC-26` and every reference page that names `files_examined` (`docs/reference/check.md` among them) describe the decided shape; each spec carries a revision row. — evidence: spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-26-urzua-mcp.md, spec:docs/reference/check.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Recorded the direction for the alias question in Done means item 1: the maintainer accepted on 2026-11-23, relayed from the gate tool's design thread, that "the shared contract is urzua's existing shape, and urzua conforms by addition only", so `examined` is added beside `files_examined` and nothing is renamed. The ADR that item asks for is still to be written. The `describe` answer is filed as `MILE-359`. | **structural** |
> | 2026-11-23 | Per-item evidence added for the three Done means items: the ADR drafted as `ADR-106` (left `Proposed`; acceptance is the maintainer's), the `examined` field and its parity test, and the spec and reference pages that describe it. `Status` is left to `urzua fix --apply`. | **structural** |
> | 2026-11-23 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 0 file-level, 3 scoped. Evidence: rust/crates/urzua-core/src/report.rs#Examined@sha256:2167ce1d8e62, rust/crates/urzua-cli/src/commands/check.rs#run_opened@sha256:32f5987fd009, rust/crates/urzua-cli/tests/agent_cli_contract_integration.rs#check_reports_examined_files_equal_to_files_examined@sha256:a4df9f3e6497 Reason: Pin the evidence this change delivers: the Examined field, the check run path and the tests that show it **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-23 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 3 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-23 | `Status` back to `InProgress`, and item 1's evidence removed: it cited `ADR-106`, which is `Proposed`, and a proposed ADR is a draft, not a recorded decision. Item 1 resolves when the maintainer decides `ADR-106`. Items 2 and 3 keep their evidence. **Why:** the independent review of the pull request that adds `examined` found `Done` resting on an undecided ADR; `urzua fix --apply` had accepted the `spec:` locator as resolved. | **structural** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/check.rs#run_opened` (lines 649-741) 32f5987fd009 -> 725ff7ebf190. Reason: Read the diff of rust/crates/urzua-cli/src/commands/check.rs against main. rust/crates/urzua-cli/src/commands/check.rs: Failure now carries a non-optional kind and has no conversion from a bare message, so a refusal states its kind where it is made; an unknown or undeclared rule id (including the did-you-mean branch), the flag conflicts and a path or --base argument that cannot be used are usage, drift_baseline is config. The claims each record cites are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
