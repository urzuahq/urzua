---
default: patch
closes: []
---

`embodiment.claim-renewal-unattested` accepts a `urzua reverify` row dated any day from the branch's
first commit to the commit that added the row, so a row written on one day and committed the next, a
row amended or fixed up into an older branch commit, and a row whose line a later commit touched are
no longer reported. A date in the future, or before the branch's first commit, is still reported.
