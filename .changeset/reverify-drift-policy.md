---
default: minor
closes: []
---

Add a per-category drift policy to `embodiment.claim-expired`: `drift: {code: block, test: block, spec: acknowledge}`.
`block` (the default for a category not named) refuses a renewal until the drifted locator is re-pinned; `acknowledge`
lets `urzua reverify --apply` renew over the drift and writes each drifted locator (a cited record by its id and its
newest revision-log date, any other file by its path, hash prefix and the commit that last changed it) with the reviewer's `--reason` into the
`Re-verified` row, after a `; Drift acknowledged:` separator, and re-pins a drifted pinned locator in the same act (old and new hash recorded); `ignore` leaves the category's drift out of renewal. An unknown category or policy is a load error
that lists the valid ones. The refusal for drift in a category that `--pin-only` cannot pin, such as a `spec:` locator,
now names the category, its policy and the `drift` setting, instead of suggesting `--pin-only`, and `reverify`'s review
lists a claim the policy lets through as renewable with `drift_acknowledged`. A configuration that declares no `drift`
renews as before.
