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

- Each limit has a named key under `harness.limits`; a configuration that declares none behaves as before, shown by a test. — evidence: code:rust/crates/urzua-core/src/harness/limits.rs@sha256:5fecde0b34a229b614511fff2af8c0f54a2ad0a70f45120d587d4c14bbc92a19, test:rust/crates/urzua-cli/tests/harness_limits_integration.rs#a_config_declaring_no_limit_behaves_as_before@sha256:49c41baf366d97659b9732337c75500ab9b58cebdda9a70c05c4a45be1a4b574
- A value below a security floor is a load error naming the key and the floor. — evidence: test:rust/crates/urzua-core/src/harness/limits.rs#each_floor_below_its_floor_is_an_error_naming_key_and_floor@sha256:cb2d9bb92b8e2939076c96c3a1907954e2dafac6d4216aa168be4e67e72dd9cd, test:rust/crates/urzua-cli/tests/harness_limits_integration.rs#a_limit_below_its_floor_or_an_unknown_limit_stops_the_load@sha256:22013fea5e550ec6ee01e90f201c788babd5b58d40d47a738564b6637e47fef8
- `SPEC-3` lists the format invariants and security floors of the harness with their reasons. — evidence: spec:docs/specs/SPEC-3-configuration.md@sha256:36a56b217f1246bc76d609cab480d6ce86965d79d78578a995ac322e381e044e
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings. — evidence: code:.urzua/config.yaml@sha256:07b2b094255c2e54b1d5306bd7efa40e7298690f1f548f546b019d1131e237cf

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
> | 2026-11-15 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Specified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/harness/limits.rs` 37aad8e672fc -> 5fecde0b34a2, `rust/crates/urzua-cli/tests/harness_limits_integration.rs#a_limit_below_its_floor_or_an_unknown_limit_stops_the_load` (lines 162-193) 6cd34d96ab97 -> 22013fea5e55, `docs/specs/SPEC-3-configuration.md` 2700efb62b41 -> 624e22cb9846. Reason: the editor hook timeout is now a floor of 120 seconds (was a range from 10) and the MCP request line has a 64 MiB ceiling; the translator waits 10 seconds for 'urzua harness limits' instead of 30, which changes its hash in settings.json; the claims about the hook, the limits and their documentation still hold **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `docs/specs/SPEC-3-configuration.md` 624e22cb9846 -> 36a56b217f12, `.urzua/config.yaml` 1690790d0bdc -> 07b2b094255c. Reason: Read the diff of each re-pinned file against main. SPEC-3-confi: the record's revision log and version gained the rows of this change and its text changed only where this PR says: SPEC-3 documents unread_markers, SPEC-25 states the findings section as a fenced text block; the pinned claim is unaffected; .urzua/config.yaml: `.urzua/bin` joins the protected paths and embodiment.back-pointer-resolution gains `unread_markers: report`; no other key changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
