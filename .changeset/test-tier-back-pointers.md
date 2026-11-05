---
default: minor
closes: []
---

`embodiment.back-pointer-resolution` takes an optional `test_paths` option declaring the test class:
a `Verifies: ADR-12` comment in a file under it is test-tier evidence, so a record whose tests cite
it computes `Verified` without a restated `test:` locator list. `embodiment.consistency` also judges
a record that has `Embodiment` but no `Realized-by` field when a source back-pointer cites it,
computing from the back-pointers and naming that evidence in the finding; a record neither cited
nor carrying `Realized-by` is still not judged. `urzua fix` repairs the same records.
