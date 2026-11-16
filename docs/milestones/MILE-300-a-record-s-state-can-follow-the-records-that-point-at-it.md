---
Stable-Id: 01M7HY6Z0VZ1BB4GCCNR71CGMF
Status: Planned
Phase: '1'
Track: engine-extensibility
Blocked-on: null
---
# 300 — A record's state can follow the records that point at it

## What

Let a configuration declare that a record type's state is computed from the records that reference it through a declared relation (for example: success when all are success, in progress when some are), so a record that groups others needs no hand-kept list of them.

## Why

An audit on 2026-11-16 read all non-test engine code for places that name a particular record type, status, field or workflow. It found that a record which groups other records can today get a computed state only by listing each member by hand in its own items; there is no way to compute a state from an inverse relation. This is a generic capability and names no record type.

## Done means

- A configuration declares, for a type and a relation, how the referencing records' states combine into the record's state, and `urzua fix` computes it, shown by a test with two invented types.
- A record with no referencing record has a declared state for that case, and the report says which records were counted.
- A configuration that declares none behaves as before.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the audit of 2026-11-16, asked for after the maintainer said the tooling has no opinion about which record types a repository defines and that this is driven by configuration, not by the code. | **structural** |
