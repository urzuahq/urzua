---
Stable-Id: 01M6J44PX8CWX5PG5JZK0Z1D70
Status: Accepted
Accepted-on: 2026-11-04
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-2
---
# 67 — Evidence pins, a drift grace window and a claim age are opt-in refinements of git-history drift

## Context

`ADR-32` made drift unconditional: a `Realized-by` locator whose last commit follows the commit that
last touched the `Realized-by` line is drifted, and the only way to clear it is to touch the line
again. `RFC-2` proposed the opposite on each point (recorded and current hashes, a grace window, an
attributed refresh) and says its content-hash detection is not built. `MILE-129` (decision recorded
2026-11-04) settles the tension: content-hash pins on evidence, a configurable grace window in days,
and a warning when a claim exceeds a configured age, each opt-in and off by default.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep drift unconditional | Nothing to build | A large corpus blocks constantly; no way to say "re-verified, still holds" |
| B. Three independent opt-ins, no new field (chosen) | Behaviour is unchanged until written; reuses the `Realized-by` line and the rule's options; core stays pure | A pin is re-set by hand; no attributed re-baseline command |
| C. Pins as a separate header field | Room for per-locator metadata | A schema change to `Realized-by`, which `ADR-18` kept flat |

## Decision

In the context of `ADR-32`'s unconditional git-history drift, facing a corpus too large to re-verify on
every unrelated commit, we decided: **evidence pins, a grace window and a claim age are three
opt-in refinements, and none changes behaviour until a repository writes it.**

- **Pin syntax.** A suffix on a locator, `code:path@sha256:<64 lowercase hex>`, the narrowest form:
  no new field and no change to the locator grammar for an unpinned locator. Any other text after
  `@sha256:` is not a pin and stays part of the path, so a malformed pin fails `embodiment.locator-exists`
  instead of reading as unpinned.
- **Pin semantics.** A pinned locator drifts when its current content hash differs, and git history is
  not consulted for it. An unpinned locator in the same record keeps the heuristic.
- **Grace window.** `drift_grace_days` on `embodiment.consistency`: drift is not reported while the
  cited file's last commit is newer than the window. It applies to both heuristic and pin drift. A
  commit git cannot date is never held back.
- **Claim age.** `claim_max_age_days` on `embodiment.consistency`: a warning when the commit that last
  touched the `Realized-by` line is older than the limit. It is an additional finding of the same rule,
  not a new rule.
- **Purity.** The CLI computes hashes, commit dates and ages and hands plain data to the pure core,
  as `ADR-32` does for drift.
- **Not decided here.** An attributed re-baseline (`urzua fix` re-pinning with a revision row) is
  not in the `MILE-129` decision and is not built; a pin is re-set by hand.

## Reversibility

Additive and off by default: two optional rule options, one optional locator suffix, one new
parameter on a pure function. Removing the options or the suffix returns the `ADR-32` behaviour.

## Consequences

- A pinned locator's drift is exact, and an unrelated commit never reports.
- A pin is checked against the working tree, so an uncommitted edit to a pinned file reports at once and
  the grace window cannot hold it back (its last commit is old).
- Without a re-baseline command, a changed pinned file is cleared by editing the hash, which has no
  attribution beyond the commit that does it.
- The grace window delays a true positive by design; it trades latency for fewer blocks.

## Amendment (2026-11-04): re-baselining is built, and three behaviours shipped since

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

Statements in the Context-to-Consequences text are no longer true, and three behaviours that change
when a record reports are not recorded. The three opt-ins as decided (pin syntax, grace window,
claim age) are built as written.

- **Attributed re-baselining is built.** "Not decided here" says an attributed re-baseline "is not
  built; a pin is re-set by hand". `urzua fix --rebaseline` (`#238`, `MILE-181`) now lists each stale
  pin with its recorded and current hash and, with `--apply`, rewrites a record's pins and appends an
  attributed revision row. It is gated: it needs `--ids` (a record path) or `--force`, an identity,
  and `--reason`. Option B's con "no attributed re-baseline command" and the Consequences bullet
  "Without a re-baseline command, a changed pinned file is cleared by editing the hash" no longer
  apply. A record with no pins to re-set (an unpinned record) is still pinned by hand, as `SPEC-8`
  describes.
- **A stated `Drift detected` is accepted inside the window (`#182`).** Within `drift_grace_days` a
  record that states `Drift detected` agrees with what is computed, and `urzua fix` leaves it alone.
- **A declared audit date moves the window's anchor (`MILE-166`).** `drift_audit_field` (here
  `Last-audited`) makes the window count from the date a record declares its evidence was audited,
  not from the cited file's last commit.
- **Git failures and shallow clones are reported (`#214`).** A commit git cannot find is an error
  the check reports, not a read of "no drift". `ADR-32`'s amendment records the same change.

## References

- `ADR-32` -- the git-history drift this refines.
- `RFC-2` -- the hash, grace window and attributed refresh proposal.
- `MILE-129` -- the decision this builds.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Amendment added: attributed re-baselining is built (`#238`, `MILE-181`), and `#182`, `MILE-166` and `#214` are recorded. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. `Status` stays `Proposed` and the Decision text is unchanged. | **substantive** |
> | 2026-11-04 | Status changed from Proposed to Accepted, including the amendment of 2026-11-04. **Why:** Maintainer decision, 2026-11-04. | **substantive** |
> | 2026-11-04 | `Implements: ADR-67` back-pointers added in `rust/crates/urzua-cli/src/discovery.rs` (the pin branch, the grace window, the audit anchor), `rust/crates/urzua-core/src/fix.rs` (`detect_rebaselines`, `apply_rebaseline`) and `rust/crates/urzua-cli/src/commands/fix.rs` (`run_rebaseline`). Comments only; no behaviour changes. **Why:** the drift review of 2026-11-04 found no back-pointer from the code to this record. | **structural** |
