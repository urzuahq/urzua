# 0021 — ULID as the stable-ID encoding

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-id/src/lib.rs, code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-id/src/lib.rs
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: ADR-0003 (Accepted)

## Context

ADR-0003 committed to separating stable identity from display numbering but deliberately deferred
the exact encoding — ULID, UUIDv7, or a short human-typable prefix scheme — as swappable
implementation detail. `urzua migrate ids` needs one concrete encoding to actually backfill.

## Decision

In the context of needing a real encoding to implement backfill against, facing three candidates
ADR-0003 left open, we decided on **ULID**: 128-bit, time-ordered, lexicographically sortable,
collision-free without coordination — exactly the property twenty concurrent agent branches need,
and the literal reference case the `ulid` crate (dylanhart/ulid-rs) is built for. Verified before
choosing: pure Rust (only `rand` under its `std` feature, no C dependency), compiles clean under the
pinned 1.87.0 toolchain, confirmed by generating and printing a real ID.

**Simplification, stated rather than silently taken**: ADR-0003 §6 says backfilled stable IDs
should derive their timestamp from git history, so chronology is preserved without coordination.
This implementation instead stamps backfilled IDs with the time of the migration run
(`Ulid::generate()`), not each record's original git history. Nothing in the schema currently reads
or sorts by a stable ID's embedded timestamp — the load-bearing property ADR-0003 actually requires
is uniqueness and lexicographic sortability *going forward*, not retroactive chronological fidelity
for records that already have working display numbers. Git-history-derived backfill timestamps are
a real, tracked follow-up, not a silently dropped requirement.

## Reversibility

ADR-0003 already named this the least-reversible piece once real cross-references exist. None do
yet — nothing currently resolves a pointer via stable ID (all resolution is filename-derived display
numbers, unchanged by this ADR) — so this is the cheapest point at which to have picked wrong.

## Consequences

- `urzua-id` gains `StableId::generate() -> StableId`, wrapping `ulid::Ulid::generate()`.
- `urzua migrate ids` backfills a `Stable-Id` header field into every record lacking one, dry-run by
  default, retaining the filename's current number as the display number unchanged (ADR-0003 §4)
  — cross-references keep resolving by filename-derived number exactly as they do today; nothing
  about `pointer.resolution` changes.
- Git-history-derived backfill timestamps remain open, tracked against this ADR, not ADR-0003 —
  revisit once something actually consumes a stable ID's embedded time ordering.

## References

- ADR-0003 — the separation this ADR picks a concrete encoding for.
- `rust/crates/urzua-id/src/lib.rs` — where `StableId` already lives, encoding left open until now.
