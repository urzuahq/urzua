---
Stable-Id: 01M6K4FN3PKBQ7282Z20FHQ18R
Status: Done
Phase: '0'
Track: embodiment-model
Implements: ADR-67
Realized-by: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#detect_lists_each_stale_pin_with_both_hashes_and_writes_nothing_observed_failing@sha256:92455c576023f6a842f66a58d2da3b4344c2e8ff8dfd7a12ff1bfa2f69daa0c5, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_re_pins_only_the_chosen_record_with_who_and_why_then_is_a_no_op_observed_failing@sha256:5c4d251e51ffe05cb113f79f727e2624dcf302077bb319f879c60de4f3fec007, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_refuses_without_a_selector_or_a_reason_and_force_is_explicit_observed_failing@sha256:68898aa6e43f6f67a7a06ba590acd188a0334e443f208f5f0d8432f7c75e9f08, spec:docs/specs/SPEC-1-cli.md, code:rust/crates/urzua-cli/src/commands/fix.rs
Blocked-on: —
---
# 181 — fix re-pins a drifted evidence pin on review with an attributed revision row

## What

`urzua fix` re-pins a drifted evidence pin after review, under its existing review-then-apply shape: detect lists each drifted pinned locator with its recorded and current hash, and `--apply` with `--ids` (or `--force`) rewrites the pin to the current hash, appends a revision-log row naming who attested and why, and refuses without an identity.

## Why

A real adopter environment re-baselines drifted evidence with an attributed refresh: list the drifted locators, apply by reviewed id, record who attested. `ADR-67` adds content-hash pins and states that "an attributed re-baseline is not in the `MILE-129` decision and is not built; a pin is re-set by hand", so a changed pinned file is cleared by editing a hash, with no attribution beyond the commit that does it. Without this a pin is a block with no sanctioned way to say "re-verified, still holds", which is what `RFC-2` set out to provide. `fix --apply` already has the write path, the lock and the identity tiers (`ADR-14`, `ADR-20`). It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

- A planted change to a pinned file in this repository's corpus is listed by `urzua fix`. — evidence: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#detect_lists_each_stale_pin_with_both_hashes_and_writes_nothing_observed_failing@sha256:92455c576023f6a842f66a58d2da3b4344c2e8ff8dfd7a12ff1bfa2f69daa0c5
- The planted change is cleared by `fix --apply --ids` with a revision-log row carrying the attested name and the reason, and a second run is a no-op. — evidence: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_re_pins_only_the_chosen_record_with_who_and_why_then_is_a_no_op_observed_failing@sha256:5c4d251e51ffe05cb113f79f727e2624dcf302077bb319f879c60de4f3fec007
- `--force` is never the default and an apply without an identity is refused. — evidence: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_refuses_without_a_selector_or_a_reason_and_force_is_explicit_observed_failing@sha256:68898aa6e43f6f67a7a06ba590acd188a0334e443f208f5f0d8432f7c75e9f08
- The pin is rewritten without disturbing the rest of the `Realized-by` line. — evidence: test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#apply_re_pins_only_the_chosen_record_with_who_and_why_then_is_a_no_op_observed_failing@sha256:5c4d251e51ffe05cb113f79f727e2624dcf302077bb319f879c60de4f3fec007
- The capability is exercised on this repository's own records, not only in unit tests. — evidence: spec:docs/specs/SPEC-1-cli.md, code:rust/crates/urzua-cli/src/commands/fix.rs

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's attributed evidence refresh has no counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: `urzua fix --rebaseline` (`SPEC-8`) lists stale `@sha256:` pins with recorded and current hash and, with `--apply --ids`/`--force`, `--by` and a required `--reason`, re-pins the chosen records and appends an attributed revision row; exercised on a clone of this repository's own corpus (`ADR-18`). Heuristic drift on an unpinned locator is not re-baselined, so `ADR-20`, `ADR-42` and `ADR-45` are untouched. `Realized-by` added and `Status` Planned -> Done: the Done-when is met (listed, cleared by `--apply --ids` with a row carrying name and reason, second run a no-op, no identity or reason refused, exercised on this corpus). `ADR-67` stays Proposed and its "not built" line is the maintainer's to amend. **Why:** the milestone's Done-when asks for the capability and its exercise on this corpus. | **substantive** |
> | 2026-11-04 | `Blocked-on` cleared. **Why:** it named `ADR-67` as Proposed, and the maintainer accepted it on 2026-11-04; this milestone was already `Done`. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
