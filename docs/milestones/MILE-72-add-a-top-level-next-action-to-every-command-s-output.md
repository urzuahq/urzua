---
Status: Done
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:3bf886d3d0c15c1b0fc8713e7c810426d41aa419546ebc28011fbd42e5e5eab6, code:rust/crates/urzua-cli/src/main.rs@sha256:c2f03d271cdf698394ce5bf9be9bc6fa6e68df50015410e1688220916b4e8bf9, test:rust/crates/urzua-cli/tests/next_action_integration.rs#every_other_command_and_a_could_not_run_emit_both_keys_as_null@sha256:ee92ece0205eb3e64f8ad793d30f866ab1518bd09345e92061c86ed9b3993042, test:rust/crates/urzua-cli/tests/next_action_integration.rs#a_mixed_fixable_and_judgment_check_suggests_urzua_fix@sha256:07cc061d66e6c6d5ed3b6b9fd8aa6c3a44c5541770f2553806653c9898a3aebf, test:rust/crates/urzua-cli/tests/mcp_integration.rs#every_tool_result_carries_both_next_action_keys@sha256:b2b90169224bcf978f5911abb672a22fdde9d528c2f4ad79fc818b778fa2de59, spec:docs/specs/SPEC-2-urzua-check.md
Stable-Id: 01M1YS1N3TFEQC307R6JQV66NZ
Phase: '1'
Track: schema-governance
Implements: ADR-23
Blocked-on: —
---
# 72 — Add a top-level next_action to every command's output

## What

A single top-level `next_action` field on every command's JSON output, alongside `status` and
`findings`: a plain string (or `null`) naming the single most useful next command to run, given
everything in this response. Distinct from MILE-44's `suggested_action`, which is scoped to one
finding at a time: `next_action` looks at the response as a whole and recommends one thing, e.g.
`"run urzua fix to auto-repair 3 of these findings"` after a `check` that found mixed
mechanically-fixable and human-judgment findings, or `null` when there's nothing better to do than
read the findings themselves.

## Why

Urzua's own stated design is that an agent acts on output directly instead of interpreting a
sentence first (README). `status` and `findings[]` already remove the need to interpret what
happened. Nothing yet removes the smaller but real step of deciding what to do about it: an agent
still has to reason from a findings list to a plan. A single recommended next command is the same
category of value `suggested_action` already provides per-finding, applied once to the whole
response instead of forcing the caller to synthesize one from several.

## Done means

- Every command's JSON output carries a top-level `next_action` (a string or `null`) beside `status` and `findings`, naming the single most useful next command given the whole response, `null` when there is nothing better than reading the findings. — evidence: code:rust/crates/urzua-core/src/report.rs@sha256:3bf886d3d0c15c1b0fc8713e7c810426d41aa419546ebc28011fbd42e5e5eab6, code:rust/crates/urzua-cli/src/main.rs@sha256:c2f03d271cdf698394ce5bf9be9bc6fa6e68df50015410e1688220916b4e8bf9, test:rust/crates/urzua-cli/tests/next_action_integration.rs#every_other_command_and_a_could_not_run_emit_both_keys_as_null@sha256:ee92ece0205eb3e64f8ad793d30f866ab1518bd09345e92061c86ed9b3993042
- A test asserts `next_action` on each command, and a mixed fixable and judgment `check` suggests `urzua fix`. — evidence: test:rust/crates/urzua-cli/tests/next_action_integration.rs#a_mixed_fixable_and_judgment_check_suggests_urzua_fix@sha256:07cc061d66e6c6d5ed3b6b9fd8aa6c3a44c5541770f2553806653c9898a3aebf, test:rust/crates/urzua-cli/tests/mcp_integration.rs#every_tool_result_carries_both_next_action_keys@sha256:b2b90169224bcf978f5911abb672a22fdde9d528c2f4ad79fc818b778fa2de59
- `SPEC-2` documents `next_action` and it appears in urzua's own `check` output. — evidence: spec:docs/specs/SPEC-2-urzua-check.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-14 | Built as the maintainer decided on 2026-11-14: `next_action` is a bare command string or `null`, joined by `next_action_reason` (a count-based sentence, `null` exactly when `next_action` is); both are on every command's JSON, always present. `check` and `audit` pick the first of `urzua fix --apply`, `urzua index --apply`, `urzua refs refresh` that an unwaived error or warning finding offers as a `run-command`; `fix` says `urzua fix --apply` after a detect run and `urzua check` after an apply; every other command and every not-run report emits both as `null`; `urzua mcp` carries them through the shared reports. `SPEC-2` documents it. **Why:** the milestone fixed the name and string shape; the reason key, the priority order and the per-command scope were left open and decided by the maintainer. | **substantive** |
> | 2026-11-14 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (3 of 3 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
