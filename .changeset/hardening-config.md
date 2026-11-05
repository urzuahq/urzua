---
default: patch
closes: []
---

`embodiment.locator-on-back-pointer-path` no longer advises dropping the `Realized-by` locator of a
record whose type counts no source comment as evidence (`statuses.delivery.back_pointer_evidence:
false`), and tells a locator whose file already carries the `Implements:` or `Verifies:` comment to
drop the locator instead of adding the comment. `closed_statuses` (`claim.status-agreement`) and
`terminal_statuses` (`narrative-field.stale`) are optional once every record type either declares
`statuses` or has no status field.
