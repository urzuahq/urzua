---
Stable-Id: 01M6K4FN3PKBQ7282Z20FHQ18R
Status: Done
Phase: '0'
Track: embodiment-model
Implements: ADR-67
Realized-by: code:rust/crates/urzua-cli/src/commands/fix.rs, code:rust/crates/urzua-core/src/fix.rs, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs
Blocked-on: ADR-67 (Proposed; the pin and the grace window it describes)
---
# 181 — fix re-pins a drifted evidence pin on review with an attributed revision row

## What

`urzua fix` re-pins a drifted evidence pin after review, under its existing review-then-apply shape: detect lists each drifted pinned locator with its recorded and current hash, and `--apply` with `--ids` (or `--force`) rewrites the pin to the current hash, appends a revision-log row naming who attested and why, and refuses without an identity.

## Why

A real adopter environment re-baselines drifted evidence with an attributed refresh: list the drifted locators, apply by reviewed id, record who attested. `ADR-67` adds content-hash pins and states that "an attributed re-baseline is not in the `MILE-129` decision and is not built; a pin is re-set by hand", so a changed pinned file is cleared by editing a hash, with no attribution beyond the commit that does it. Without this a pin is a block with no sanctioned way to say "re-verified, still holds", which is what `RFC-2` set out to provide. `fix --apply` already has the write path, the lock and the identity tiers (`ADR-14`, `ADR-20`). It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

## Done means

A planted change to a pinned file in this repository's corpus is listed by `urzua fix`, cleared by `fix --apply --ids` with a revision-log row carrying the attested name and the reason, and a second run is a no-op; `--force` is never the default and an apply without an identity is refused; the pin is rewritten without disturbing the rest of the `Realized-by` line; and the capability is exercised on this repository's own records, not only in unit tests.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's attributed evidence refresh has no counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
> | 2026-11-04 | Built: `urzua fix --rebaseline` (`SPEC-8`) lists stale `@sha256:` pins with recorded and current hash and, with `--apply --ids`/`--force`, `--by` and a required `--reason`, re-pins the chosen records and appends an attributed revision row; exercised on a clone of this repository's own corpus (`ADR-18`). Heuristic drift on an unpinned locator is not re-baselined, so `ADR-20`, `ADR-42` and `ADR-45` are untouched. `Realized-by` added and `Status` Planned -> Done: the Done-when is met (listed, cleared by `--apply --ids` with a row carrying name and reason, second run a no-op, no identity or reason refused, exercised on this corpus). `ADR-67` stays Proposed and its "not built" line is the maintainer's to amend. **Why:** the milestone's Done-when asks for the capability and its exercise on this corpus. | **substantive** |
