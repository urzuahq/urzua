---
Stable-Id: 01M83NH9B10G0ZA0XXVPFGJT20
Status: Planned
Phase: '1'
Track: integrations
Implements: ADR-103, RFC-50
Blocked-on: 
---
# 361 — The export carries a format version and the config-declared data a client needs, with a published schema

## What

`urzua export --format json` carries a format version and the data a client now has to take from the configuration or from Rust source: each record type's declared statuses and status roles, and its declared sections. A JSON Schema for the export is published. Every field is an addition to the existing export; no key is renamed or removed.

## Why

The board leaves the engine and becomes the first ecosystem client (`RFC-27` §3 and its 2026-11-08 amendment class a generated presentation of the records as ecosystem; `RFC-50` §4; `ADR-103`). The maintainer chose the order on 2026-11-23: "Export first, then client (Recommended)", so the client reads only published JSON. Today the board reads declared data that the export does not carry, in `rust/crates/urzua-cli/src/commands/board_declared.rs` (the key lists and the per-type declarations behind its prompt and its "About this board" panel), and the export has no version field (`RFC-50` §6).

This is the export's half of the work. `MILE-359` gives every report a `schema_version` and a describe answer, by addition; this milestone needs the export to carry that version and does not duplicate it. It could be written as added Done means items on `MILE-359`. It is filed separately because its content (the declared types, status roles and sections, and the export's own schema) is not part of what `MILE-359` plans, and because `MILE-359`'s items wait on a contract release that this does not.

## Done means

- The export's JSON carries `schema_version` with the value `MILE-359` assigns, shown by a test on `urzua export --format json`; if `MILE-359` has not landed, the export carries it here and `MILE-359`'s report-wide item reuses the same value.
- The export carries, for each record type, the declared statuses with their roles (the lists the engine reads as progress, success, failure and parking) and the declared sections, shown by a test on two fixture configurations of different shape and on this repository's own export.
- Every datum `board_declared.rs` reads from `Config` is present in the export, shown by a test that fails when the board's declared-data function is given only the export's JSON and returns something different from the one built from `Config`.
- A JSON Schema for the export is published under `docs/reference/` and a test validates this repository's own export against it; a key added to the export without a schema entry fails the test.
- No existing export key is renamed or removed, shown by a test over the key lists present today.
- `docs/reference/` documents the export's version and the new declared data; the specification that describes the export carries a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-23 | Filed as planned work: nothing built, no evidence yet. Filed from the maintainer's sequencing answer of 2026-11-23, "Export first, then client (Recommended)". | **structural** |
