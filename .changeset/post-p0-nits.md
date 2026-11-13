---
default: patch
closes: []
---

`embodiment.claim-renewal-unattested` says an anchor field "is empty" when the branch left it
blank, and keeps "was removed" for a field that is gone. An unreadable merge-base copy of a deleted
record is warned about once per path, under the rule that reached it, instead of twice under
`diff.decider-required`.
