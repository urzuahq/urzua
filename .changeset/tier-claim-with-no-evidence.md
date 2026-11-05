---
default: minor
closes: []
---

`embodiment.consistency` now reports a record that states `Verified` or `Implemented` while having no `Realized-by` field and no source or test back-pointer citing it: a claim with no evidence of either kind is reported against the computed `Not started` instead of passing silently. A record that states any other value with no evidence is still not judged and is counted as unexamined.
