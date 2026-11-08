---
Stable-Id: 01M6R3NK4BHT2X4XK5A4F5X3S9
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-id/src/lib.rs@sha256:ff0e901149329c86d19adab473a3ce5593980001b8385c405472b17b3273884b, code:rust/crates/urzua-cli/src/commands/migrate.rs@sha256:f3fe484ef1e800cafe0af678597102d62432a421e15f176981aacf4e8d20622f, test:rust/crates/urzua-id/src/lib.rs@sha256:ff0e901149329c86d19adab473a3ce5593980001b8385c405472b17b3273884b
Date: 2026-09-05
Accepted-on: 2026-09-05
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-3
---
# 21 — ULID as the stable-ID encoding

## Context

ADR-3 committed to separating stable identity from display numbering but deliberately deferred
the exact encoding — ULID, UUIDv7, or a short human-typable prefix scheme — as swappable
implementation detail. `urzua migrate ids` needs one concrete encoding to actually backfill.

## Decision

In the context of needing a real encoding to implement backfill against, facing three candidates
ADR-3 left open, we decided on **ULID**: 128-bit, time-ordered, lexicographically sortable,
collision-free without coordination — exactly the property twenty concurrent agent branches need,
and the literal reference case the `ulid` crate (dylanhart/ulid-rs) is built for. Verified before
choosing: pure Rust (only `rand` under its `std` feature, no C dependency), compiles clean under the
pinned 1.87.0 toolchain, confirmed by generating and printing a real ID.

**Simplification, stated rather than silently taken**: ADR-3 §6 says backfilled stable IDs
should derive their timestamp from git history, so chronology is preserved without coordination.
This implementation instead stamps backfilled IDs with the time of the migration run
(`Ulid::generate()`), not each record's original git history. Nothing in the schema currently reads
or sorts by a stable ID's embedded timestamp — the load-bearing property ADR-3 actually requires
is uniqueness and lexicographic sortability *going forward*, not retroactive chronological fidelity
for records that already have working display numbers. Git-history-derived backfill timestamps are
a real, tracked follow-up, not a silently dropped requirement.

## Reversibility

ADR-3 already named this the least-reversible piece once real cross-references exist. None do
yet — nothing currently resolves a pointer via stable ID (all resolution is filename-derived display
numbers, unchanged by this ADR) — so this is the cheapest point at which to have picked wrong.

## Consequences

- `urzua-id` gains `StableId::generate() -> StableId`, wrapping `ulid::Ulid::generate()`.
- `urzua migrate ids` backfills a `Stable-Id` header field into every record lacking one, dry-run by
  default, retaining the filename's current number as the display number unchanged (ADR-3 §4)
  — cross-references keep resolving by filename-derived number exactly as they do today; nothing
  about `pointer.resolution` changes.
- Git-history-derived backfill timestamps remain open, tracked against this ADR, not ADR-3 —
  revisit once something actually consumes a stable ID's embedded time ordering.

## References

- ADR-3 — the separation this ADR picks a concrete encoding for.
- `rust/crates/urzua-id/src/lib.rs` — where `StableId` already lives, encoding left open until now.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit c63eebe25. | **structural** |
> | 2026-11-04 | Pinned `rust/crates/urzua-cli/src/commands/migrate.rs` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-verified: the only change since the evidence was last checked is the removal of the blockquote and bold-list Stable-Id shapes from migrate ids; ULID generation (urzua-id) is unchanged and the Decision holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs` 0d031cec3105 -> 772ecad047bc. Reason: Read the diff of each pinned span since the old pin and the claim still holds. commands/migrate.rs: gained `migrate config` (config schema upgrade), a partial-failure status for migrate ids, the fix lock and a re-read before each write, and Failure error values, while migrate ids still backfills the Stable-Id. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-id/src/lib.rs@sha256:d7d213ab85fc, rust/crates/urzua-id/src/lib.rs@sha256:d7d213ab85fc Reason: Pinned the files as they are now after checking each against the record it evidences: header.rs still parses the declared YAML frontmatter shape (ADR-17), fix.rs still has detect_repairs (ADR-19), urzua-id still builds ULID stable ids (ADR-21), main.rs still prints JSON with no --format flag on check (ADR-23), urzua-io still resolves the attesting identity in the ADR-31 order, scripts/generate-dashboard.py still generates the static dashboard (ADR-37), report.rs still defines Report and Notice and main.rs still emits them (ADR-46), publish-release.yml still verifies the tree it ships through checks.yml (ADR-47), knope.toml still releases from changesets (ADR-48), and ci.yml still gates changesets on .github/release-paths (ADR-49). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-id/src/lib.rs` d7d213ab85fc -> ff0e90114932. Reason: urzua-id/src/lib.rs: removed the unused DisplayNumber type and urzua_id::Error enum (and the thiserror dependency); StableId and StableId::generate, the claim ADR-21 makes about ULID generation, are unchanged and its generate tests still pass **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs` 772ecad047bc -> f3fe484ef1e8. Reason: split run_ids_inner, moving per-record work into backfill_stable_id (migrate.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
