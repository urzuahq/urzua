---
default: patch
---

An absent `claim_paths` directory no longer aborts `check`. Git keeps no empty directory, so a
declared `.changeset` ceases to exist the moment a release consumes the last fragment, and a
repository following the documented pattern lost `check` entirely. The absence is reported as a
notice, which is visible and never moves the exit code; a path that is actively wrong — a file where a
directory was declared, or a link resolving outside the repository — still aborts.
