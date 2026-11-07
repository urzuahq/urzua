---
default: patch
closes: []
---

`urzua rules` labels `embodiment.locator-promotion-candidate` as `anchor-scoped`, and with
`scope: changed` the rule reports a shared locator on every changed record that cites it, not only
the first, so `check <path>` and a file-scoped waiver each see the finding on their own record.
