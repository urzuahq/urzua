---
default: patch
closes: []
---

The commands that read the corpus now share one startup path (repository root, config, tracked-file
discovery, record loading) and one could-not-run report. No behavior change: every command's JSON
output and exit codes, including the exit-2 could-not-run results for a missing repository, a missing
config and an unreadable record, are identical.
