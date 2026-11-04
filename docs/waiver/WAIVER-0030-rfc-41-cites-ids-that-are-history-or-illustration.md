---
Stable-Id: 01M6JRM02F7QS38SNB6NN4PWFX
Rule: citation.resolution
Scope: docs/rfc/RFC-41-a-source-comment-citing-a-bug-or-adr-id-is-a-claim-the-engine-should-be-able-to-check.md
Reason: "The RFC's motivating incident is a bug record that was created, deleted and renumbered, and source comments that kept citing the old number. The old id is the evidence the proposal argues from and does not resolve by design. Reviewed finding by finding; expires so the exception is revisited."
Expires: 2027-11-04
---
# WAIVER-30 — RFC-41 cites ids that are history or illustration

## What this covers

`citation.resolution` reports ids cited in RFC-41's prose that resolve to no record. The RFC's motivating incident is a bug record that was created, deleted and renumbered, and source comments that kept citing the old number. The old id is the evidence the proposal argues from and does not resolve by design.

## References

- `docs/rfc/RFC-41-a-source-comment-citing-a-bug-or-adr-id-is-a-claim-the-engine-should-be-able-to-check.md` — the record whose text is covered.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed with the own-repo rollout of `citation.resolution` (`MILE-156`). | **substantive** |
