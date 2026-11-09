---
Status: Planned
Stable-Id: 01M1Y5JXTAAYXW2H2QPM728ZAA
Phase: '1'
Track: corpus-corrections
Implements: RFC-1, RFC-5, RFC-8, RFC-11
Blocked-on: —
---
# 33 — Fold corrections into existing RFCs

## What

Several specific, evidenced corrections not yet routed: RFC-8's repair-vs-lowered-claim gap, RFC-5's shared last-audited-timestamp coupling, a declared expected-topology opt-out for warn-only checks, a declared branch scope for coherence checks, a mandatory planted-violation-test requirement for absence-checks (codified, not just practiced), and RFC-1's transitions-list-vs-flat-field schema question.

## Why

Each is a specific correction to an existing RFC that no public record carries yet -- routing them is cheaper now than after more schema work builds on the unpatched versions.

## Done means

- The `RFC-8` repair-versus-lowered-claim correction is either folded into the named RFC with a revision row or recorded as declined with the reason.
- The `RFC-5` shared last-audited timestamp correction is either folded into the named RFC with a revision row or recorded as declined with the reason.
- The declared expected-topology opt-out correction is either folded into the named RFC with a revision row or recorded as declined with the reason.
- The declared branch scope for coherence checks correction is either folded into the named RFC with a revision row or recorded as declined with the reason.
- The planted-violation-test requirement for absence-checks correction is either folded into the named RFC with a revision row or recorded as declined with the reason.
- `RFC-1`'s transitions-list schema question is either folded into the named RFC with a revision row or recorded as declined with the reason.
- None of these corrections is left unrouted.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-09 | Removed an appeal to research a reader cannot open; the argument now rests on the corrections themselves, each named in What. `Status` unchanged. | **structural** |
> | 2026-11-09 | Retitled "Fold corrections into existing RFCs" (filename slug and H1) so the title no longer points at a source a reader cannot open. `Stable-Id` is unchanged, so every reference resolves; no other record cited the old filename. `Status` unchanged. | **structural** |
