# 31 — Accept RFC-2: `urzua fix` already is the detect/apply split; fix identity-resolution priority

> Status: Accepted
> Embodiment: Verified
> Realized-by: code:rust/crates/urzua-io/src/lib.rs, test:rust/crates/urzua-io/src/lib.rs
> Date: 2026-09-06
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: RFC-2 (Accepted)

## Context

RFC-2 was written against a data model that never shipped — per-claim hash IDs
(`adr-0069/realized_by/0`), a `refresh` command, content-hash drift detection. What actually shipped
(ADR-18's categorized-locator Embodiment MVP, ADR-19/20's `urzua fix`, RFC-8's
re-verify-before-write) satisfies most of RFC-2's *structural* proposal already, under different
names, decided independently rather than by revisiting this RFC:

- **Proposal §1 (detect/apply as separate operations)** — this is exactly `urzua fix` (read-only) vs.
  `fix --apply --ids <list>` (scoped) vs. `fix --apply --force` (bypass), decided in ADR-19/20
  before this RFC was ever revisited.
- **Open question "does apply need to re-verify at write time"** — resolved by RFC-8 §4, already
  implemented: `run_fix` re-reads each file from disk immediately before writing and refuses if the
  value it expected to replace has already changed.
- **Proposal §3 (no interactive confirm-prompt)** — `fix` was never built with one; the JSON-report
  and `--ids` design was chosen the same way for the same reason, independently.

One real discrepancy surfaced comparing the RFC against the actual code: RFC-2 §2 specifies
identity resolving from a **verified source first**, with free-text `--by` only as a last resort —
"raising the cost of an accidental or careless rubber-stamp." `urzua_io::resolve_identity` shipped
with the opposite order: explicit `--by` checked first, `gh api user` and `git config user.name` only
as fallbacks. That inverts the accountability property the RFC exists to establish — a caller can
always type any name and skip the verified-source check entirely, which is exactly the "indistinguishable
from a truthful attestation" failure the RFC's motivating scenario describes.

One real gap remains unaddressed by anything shipped: RFC-2's original motivating scenario was
**content changed since last verified**, detected by re-hashing a locator and diffing against a
recorded value. Nothing computes or stores that hash today — `embodiment.consistency` checks whether
a locator exists (and whether it's a `test:`-category locator), never whether its content drifted
since the record was marked `Verified`. This is a materially different, still-open mechanism.

## Decision

In the context of a Draft RFC whose structural proposal mostly already shipped under other decisions,
facing one real discrepancy and one real gap, we decided:

1. **Accept RFC-2.** Its detect/apply/force-bypass model is satisfied by `urzua fix`; its
   re-verify-at-apply-time question is satisfied by RFC-8 §4. No new command or flag needed.
2. **Fix `resolve_identity`'s priority order** to match the RFC's actual intent: `gh api user` first,
   then `git config user.name`, then explicit `--by` only when neither verified source is available.
   An explicit `--by` can no longer silently override a verified login.
3. **Add a timeout to the `gh api user` subprocess call.** `Command::output()` blocks indefinitely if
   `gh` hangs (an unauthenticated interactive prompt, a stalled network call) — the exact "gh api user
   requires a network call... what's the right timeout/failure behavior when it hangs" the RFC left
   undesigned. Decided: a bounded wait, falling through to `git config user.name` on timeout rather
   than hanging `fix --apply` indefinitely.
4. **Content-hash drift detection is explicitly not built here** — it is a real, distinct future `fix`
   tier (already named in the backlog as Tier 2/3), not something this ADR retrofits onto the
   existing Tier 1 Embodiment check. RFC-2 stays the record of what that mechanism looks like when
   it's built.
5. **The remaining two RFC-2 open questions are resolved as: no.** `detect`'s output is not written
   to a stable file path — `fix`'s architecture makes re-running it as cheap as reading a cached file
   would be, so a persisted artifact adds a staleness risk (a stale `drift.json` someone trusts instead
   of re-running) without a real benefit over just re-running `fix`.

## Reversibility

The identity-resolution reorder is a behavior change for anyone relying on `--by` silently winning
over a `gh`-authenticated session — narrow, and arguably a bug fix rather than a breaking change,
since the original intent (per RFC-2, written before `resolve_identity` was implemented) was
always verified-first. The timeout is additive robustness, no behavior change on the success path.

## Consequences

- `fix --apply` in an environment with an authenticated `gh` session now always attributes to that
  login, even if `--by` is also passed — matching RFC-8/ADR-20's original accountability intent
  more precisely than the shipped code did.
- An operator who wants to attribute a fix to someone other than their own `gh` login (e.g. applying
  on someone else's behalf with their explicit sign-off) can no longer do so via `--by` in an
  authenticated session — a real, deliberate restriction, not an oversight.
- Content-hash drift detection remains a named gap: `check`/`fix` can tell you a locator exists, not
  that it still says what it said when the record was last marked `Verified`.

## References

- RFC-2 — accepted by this ADR.
- ADR-18/19/20 — the decisions that already satisfied most of RFC-2's proposal.
- RFC-8 §4 — the re-verify-before-write decision that satisfied RFC-2's second open question.
- `rust/crates/urzua-io/src/lib.rs` — `resolve_identity`.
