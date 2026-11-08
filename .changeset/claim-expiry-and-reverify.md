---
default: minor
closes: []
---

Add per-type claim expiry and `urzua reverify`. The opt-in rule `embodiment.claim-expired` expires
a claim after a period declared per record type, status and evidence tier (`periods`), at its own
level, counting from a recorded audit date (`anchor_field`) or the commit that last touched the
`Realized-by` line; `spread_days` lengthens the first expiry by a deterministic offset from the
record's `Stable-Id` so a corpus rewritten on one day does not expire on one day. The companion
rule `embodiment.claim-renewal-unattested` reports an audit date that moved with no `Re-verified`
revision row. `urzua reverify` is the one attributed way to renew a claim: it re-resolves and
re-hashes every locator, refuses changed evidence, pins `code:` and `test:` locators that have no
pin (`--pin-only` pins them and renews nothing), caps a batch at `max_batch` records under one
shared reason, and records who renewed the claim and why. A configuration that declares neither
rule loads and reports as before. The harness bundle gains a `/reverify` command and a `reverify` verb in its helper script,
which refuse an empty reason, list the review and print the `Pinned:` and `Re-verified:` lines for the pull request body.
