---
Status: Planned
Stable-Id: 01M1Y5JP5GX3QXNQVKN2YGEY43
Phase: '3'
Track: accountability-identity
Implements: RFC-5
Blocked-on: —
---
# 19 — Human attestation with expiry (verification tier 3)

## What

A decider periodically re-affirms a decision that can't be checked mechanically, on a declared cadence.

## Why

RFC-5 §2's tier 5 names this for infra_manual/process locators; the mechanism, cadence, and workflow are undesigned.

## Done means

A declared cadence on a decision whose evidence cannot be checked mechanically makes `check` report the decision once its last human attestation is older than the cadence, an attestation records who and when in a checked field, and a fixture shows an expired attestation reported and a fresh one clean; the mechanism is exercised in urzua's own config and corpus.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |