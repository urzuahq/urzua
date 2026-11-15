---
Stable-Id: 01M7FVW4XSZBRWZCZPDGJ7SNYG
Status: Planned
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

- Each limit has a named key under `harness.limits`; a configuration that declares none behaves as before, shown by a test.
- A value below a security floor is a load error naming the key and the floor.
- `SPEC-3` lists the format invariants and security floors of the harness with their reasons.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
