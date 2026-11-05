---
default: minor
closes: []
---

`embodiment.consistency` gains three opt-in refinements of drift, all off by default. A `Realized-by`
locator written `code:path@sha256:<hash>` is pinned to that content hash and drifts only when the file's
hash differs. The `drift_grace_days` rule option holds back drift whose cited file changed fewer than
that many days ago. The `claim_max_age_days` option warns when a record's `Realized-by` line was last
touched longer ago than that. A repository that sets none of them sees no change.
