---
Stable-Id: 01M6N5F92SMEKRAQMT0FF3B1HB
Status: Done
Realized-by: code:rust/crates/urzua-core/src/rules/sequence.rs, test:rust/crates/urzua-cli/tests/sequencing_integration.rs, test:rust/crates/urzua-cli/tests/roadmap_integration.rs
Phase: '0'
Track: roadmap-tracking
Implements: RFC-48
Depends-on: ADR-82
Blocked-on: —
---
# 192 — sequence.cycle reads roadmap.pairs so check also reports a loop that crosses pairs

## What

`sequence.cycle` gains a mode that builds one graph over the pairs `roadmap.pairs` names (`ADR-82`), so `urzua check` reports a loop that crosses declared sequencing pairs, with a shortest cycle through each member as it does for one pair today. The per-pair mode stays for a repository that declares no `roadmap.pairs`.

## Why

`ADR-76` left a loop across two acyclic pairs undetected, and `ADR-82` makes the roadmap one graph over the declared pairs, so the roadmap would see a loop that `check` does not. The design re-check of `ADR-82` on 2026-11-05 asked for the follow-up so the gate and the roadmap agree.

## Done means

- A fixture with two declared sequencing pairs and a loop that uses an edge of each is reported by `urzua check` on every member, and is not reported when `roadmap.pairs` is absent.
- A pair named in `roadmap.pairs` that is not `acyclic` is a config load error.
- It is exercised in urzua's own `.urzua/config.yaml` and corpus, and `SPEC-2` and `SPEC-3` document the mode with revision rows.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** design re-check of `ADR-82` (Proposed) found `check` still sees cycles per pair while the roadmap sees them across declared pairs; this closes the gap. Depends on `ADR-82` being Accepted and on the `roadmap.pairs` key it declares. | **substantive** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` added. **Why:** Done means is met. With `roadmap.pairs` declared, `sequence.cycle` reads those pairs as one graph, the one `urzua roadmap` reads (`merged_graph` in `sequence.rs`, now shared by both), and reports a loop that uses an edge of each of two pairs on every member, with the cycle's path and the pairs it crosses; the same fixture is not reported when `roadmap.pairs` is absent or names only one of the pairs (`sequencing_integration.rs`), and `roadmap_integration.rs` shows `check` and the roadmap agreeing on a cross-pair loop. A `roadmap.pairs` name that is not `acyclic` was already a load error (`validate_roadmap`, from `MILE-182`); `sequencing_integration.rs` now covers it through `check`. Urzua's own config declares `roadmap.pairs: [sequencing]` and `urzua check` reports no cycle on its corpus. `SPEC-2` and `SPEC-3` document the mode. Built to `ADR-82`. | **substantive** |
