---
Stable-Id: 01M6N5F92SMEKRAQMT0FF3B1HB
Status: Done
Realized-by: test:rust/crates/urzua-cli/tests/sequencing_integration.rs#a_loop_across_two_declared_pairs_is_reported_on_every_member@sha256:b310fe144e5b9c8b2378be4ccf80622648149389c77a578ee48cbf7b22884965, test:rust/crates/urzua-cli/tests/roadmap_integration.rs#a_loop_across_two_declared_pairs_is_a_cycle_that_check_and_the_roadmap_both_report@sha256:909229ea7ac0f00d6092aa4e2acfe50d1e803fe90f19d49a0e01c70f37230581, test:rust/crates/urzua-cli/tests/sequencing_integration.rs#the_cross_pair_loop_is_not_reported_when_roadmap_pairs_is_absent@sha256:cc6e75e048ef0018aee1638049afaeb37aa9b6aa112c15fedbeb85b349ddf126, test:rust/crates/urzua-cli/tests/sequencing_integration.rs#a_roadmap_pair_that_is_not_acyclic_fails_check_to_load@sha256:ba087c3ed411338d12c7d81757b88fc3fc2d77ffa8830e75207b7638a90dd815, code:rust/crates/urzua-core/src/rules/sequence.rs, code:.urzua/config.yaml#roadmap@sha256:e64b8917639b89b4d97bea43420f9d6b566a37405176bb194e7fa4e4ca15e326, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md
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

- A fixture with two declared sequencing pairs and a loop that uses an edge of each is reported by `urzua check` on every member. — evidence: test:rust/crates/urzua-cli/tests/sequencing_integration.rs#a_loop_across_two_declared_pairs_is_reported_on_every_member@sha256:b310fe144e5b9c8b2378be4ccf80622648149389c77a578ee48cbf7b22884965, test:rust/crates/urzua-cli/tests/roadmap_integration.rs#a_loop_across_two_declared_pairs_is_a_cycle_that_check_and_the_roadmap_both_report@sha256:909229ea7ac0f00d6092aa4e2acfe50d1e803fe90f19d49a0e01c70f37230581
- The same fixture is not reported when `roadmap.pairs` is absent. — evidence: test:rust/crates/urzua-cli/tests/sequencing_integration.rs#the_cross_pair_loop_is_not_reported_when_roadmap_pairs_is_absent@sha256:cc6e75e048ef0018aee1638049afaeb37aa9b6aa112c15fedbeb85b349ddf126
- A pair named in `roadmap.pairs` that is not `acyclic` is a config load error. — evidence: test:rust/crates/urzua-cli/tests/sequencing_integration.rs#a_roadmap_pair_that_is_not_acyclic_fails_check_to_load@sha256:ba087c3ed411338d12c7d81757b88fc3fc2d77ffa8830e75207b7638a90dd815, code:rust/crates/urzua-core/src/rules/sequence.rs
- It is exercised in urzua's own `.urzua/config.yaml`, which declares `roadmap.pairs` and enables `sequence.cycle`, over its own corpus. — evidence: code:.urzua/config.yaml#roadmap@sha256:e64b8917639b89b4d97bea43420f9d6b566a37405176bb194e7fa4e4ca15e326
- `SPEC-2` and `SPEC-3` document the mode with revision rows. — evidence: spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-3-configuration.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** design re-check of `ADR-82` (Proposed) found `check` still sees cycles per pair while the roadmap sees them across declared pairs; this closes the gap. Depends on `ADR-82` being Accepted and on the `roadmap.pairs` key it declares. | **substantive** |
> | 2026-11-05 | `Status` Planned -> Done; `Realized-by` added. **Why:** Done means is met. With `roadmap.pairs` declared, `sequence.cycle` reads those pairs as one graph, the one `urzua roadmap` reads (`merged_graph` in `sequence.rs`, now shared by both), and reports a loop that uses an edge of each of two pairs on every member, with the cycle's path and the pairs it crosses; the same fixture is not reported when `roadmap.pairs` is absent or names only one of the pairs (`sequencing_integration.rs`), and `roadmap_integration.rs` shows `check` and the roadmap agreeing on a cross-pair loop. A `roadmap.pairs` name that is not `acyclic` was already a load error (`validate_roadmap`, from `MILE-182`); `sequencing_integration.rs` now covers it through `check`. Urzua's own config declares `roadmap.pairs: [sequencing]` and `urzua check` reports no cycle on its corpus. `SPEC-2` and `SPEC-3` document the mode. Built to `ADR-82`. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
