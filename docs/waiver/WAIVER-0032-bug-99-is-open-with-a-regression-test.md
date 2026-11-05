---
Stable-Id: 01M6NB50HKSBZ2P441CYMJ64PK
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-99-urzua-audit-exits-2-on-the-config-urzua-init-writes-for-a-corpus-with-no-type-prefix.md
Reason: "BUG-99 is deliberately open although its Realized-by names a passing test. The test pins the mitigation (init warns when its proposed config leaves audit with nothing declared) and the original reproduction no longer reproduces, but the root cause is unbuilt: audit still owns a hardcoded rule set, and RFC-38's implementation is what closes the record. Expires so the exception is revisited; the stale-waiver rule reports it once the record is Fixed."
Expires: 2026-12-31
---
# WAIVER-0032 — BUG-99 is open with a regression test

## What this covers

`status.delivery-agreement` reports `BUG-99` because it is `Open` while its `Realized-by` names a test
that exists, which computes to `Verified`. The record is open on purpose: the test covers the mitigation,
and the root cause (`audit` owning a rule set at all) is `RFC-38`'s implementation, not built. `ADR-84`
expresses a deliberately open record with evidence as this waiver, not as a "partial" status.

## References

- `docs/bugs/BUG-99-urzua-audit-exits-2-on-the-config-urzua-init-writes-for-a-corpus-with-no-type-prefix.md` -- the record whose status is waived.
- `ADR-84`, which decides the waiver as the path for a record that is deliberately open with evidence.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the enabling of `statuses.delivery` on the bug type (`ADR-84`): the one bug that is open with evidence. | **substantive** |
