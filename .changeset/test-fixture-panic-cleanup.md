---
default: patch
closes: []
---

Test-suite hygiene only: the remaining unit and integration test fixtures that were removed on the success path only are now removed when the test ends, including when it fails. No change to the `urzua` binary's behavior.
