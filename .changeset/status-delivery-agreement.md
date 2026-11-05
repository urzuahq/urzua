---
default: minor
closes: [MILE-183]
---

A record type can hold its `Status` to delivery evidence. `statuses.delivery` declares which computed
evidence tiers mean the work was delivered, which tiers mean nothing was, which statuses mean not done,
and the success status to set. The opt-in rule `status.delivery-agreement` reports a disagreement both
ways: a record whose `Realized-by` locators or citing source back-pointers show it delivered while its
`Status` reads not done, and a success `Status` with no evidence behind it. `urzua fix` proposes the
success status for the first case and `--apply` writes it with a structural revision-log row; it never
downgrades the second, which it reports as a notice. `fix` now also runs when only this rule is enabled.
