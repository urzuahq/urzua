---
Status: Planned
Stable-Id: 01M1Y5JXTAAYXW2H2QPM728ZAA
Phase: '1'
Track: corpus-corrections
Implements: RFC-1, RFC-5, RFC-8, RFC-11
Blocked-on: —
---
# 33 — Fold research corrections into existing RFCs

## What

Several specific, evidenced corrections not yet routed: RFC-8's repair-vs-lowered-claim gap, RFC-5's shared last-audited-timestamp coupling, a declared expected-topology opt-out for warn-only checks, a declared branch scope for coherence checks, a mandatory planted-violation-test requirement for absence-checks (codified, not just practiced), and RFC-1's transitions-list-vs-flat-field schema question.

## Why

Each is a real, evidenced finding from research with no home in the public corpus yet -- routing them is cheaper now than after more schema work builds on the unpatched versions.

## Done means

Each correction listed in the What (`RFC-8` repair versus lowered claim, `RFC-5` shared last-audited timestamp, the declared expected-topology opt-out, the declared branch scope for coherence checks, the planted-violation-test requirement for absence-checks, and `RFC-1`'s transitions-list schema question) is either folded into the named RFC with a revision row or recorded as declined with the reason, and none is left unrouted.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |