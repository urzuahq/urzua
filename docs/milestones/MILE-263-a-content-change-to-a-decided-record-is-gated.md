---
Stable-Id: 01M7FW40TT28BVD20M1BYN0X1A
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 263 — A content change to a decided record is gated

## What

Gate a change to the decided text of a record at an accepted status: a diff that alters anything outside an added amendment section or an added revision-log row needs a decider, as a status move does.

## Why

The adversarial review of the acceptance gate on 2026-11-15 found no way to reach Accepted without a decider, but reproduced that the whole Decision text of an Accepted record can be rewritten with the status untouched and CI green. `ADR-89` leaves in-place edits to the amendment convention, which no rule enforces.

## Done means

- A decision record amends `ADR-89` to state what part of a decided record is protected.
- A pull request that rewrites the Decision section of an Accepted record without a decider gets a blocking finding, shown by a test.
- A pull request that only adds an amendment section and a revision-log row passes.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
