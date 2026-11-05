---
default: patch
---

Test-suite hygiene only: integration-test fixture directories are now removed when each test ends, including on failure, and CI fails when a test run leaves a `urzua-*` entry in the temp directory. No change to the `urzua` binary's behavior.
