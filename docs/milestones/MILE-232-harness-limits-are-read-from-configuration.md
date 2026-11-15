---
Stable-Id: 01M7FVW4XSZBRWZCZPDGJ7SNYG
Status: Done
Phase: '1'
Track: schema-governance
Blocked-on: null
---
# 232 — Harness limits are read from configuration

## What

Move the agent harness's hard-coded limits (hook timeout, paths and records per call, depths, lengths, the gate walk cap, the MCP line size) into a `harness.limits` block, with today's value as default and, for the input caps, as a floor configuration cannot go below.

## Why

`ADR-101`'s amendment of 2026-11-15 makes a hard-coded value that shapes what a user or agent can do a defect unless it is a format invariant or a security floor. the audit of 2026-11-15 found fourteen such values in the harness.

## Done means

- Each limit has a named key under `harness.limits`; a configuration that declares none behaves as before, shown by a test. — evidence: code:rust/crates/urzua-core/src/harness/limits.rs@sha256:37aad8e672fcba92cc179c886fd7b1c4887bb67c4060d8fbee87b38bf4c658b9, test:rust/crates/urzua-cli/tests/harness_limits_integration.rs#a_config_declaring_no_limit_behaves_as_before@sha256:49c41baf366d97659b9732337c75500ab9b58cebdda9a70c05c4a45be1a4b574
- A value below a security floor is a load error naming the key and the floor. — evidence: test:rust/crates/urzua-core/src/harness/limits.rs#each_floor_below_its_floor_is_an_error_naming_key_and_floor@sha256:cb2d9bb92b8e2939076c96c3a1907954e2dafac6d4216aa168be4e67e72dd9cd, test:rust/crates/urzua-cli/tests/harness_limits_integration.rs#a_limit_below_its_floor_or_an_unknown_limit_stops_the_load@sha256:6cd34d96ab97edb588c99a1823c88b4f9e724cd497338077ac841a2f6aadb938
- `SPEC-3` lists the format invariants and security floors of the harness with their reasons. — evidence: spec:docs/specs/SPEC-3-configuration.md@sha256:2700efb62b41c5eb359587b8a746e54c6b93f5ee97071ba86025f32c9757fa15
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings. — evidence: code:.urzua/config.yaml@sha256:1690790d0bdcb8446fa7c3ce6bb48d2114d1a330744fdfa6b32b25b8206f060e

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-15 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
