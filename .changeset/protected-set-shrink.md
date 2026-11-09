---
default: patch
closes: []
---

`urzua harness run post-command` now reports a command that removes the configuration file, or any
other path, from the `authority` path gates. A changed protected path is judged against the gates the
session started with as well as the edited configuration, so the removal is reported as a changed
protected path and as a shrunk protected set, and files under a removed gate are still compared.
`urzua harness run session-start` reports a protected set that shrank since the previous session
before it takes a new baseline.
