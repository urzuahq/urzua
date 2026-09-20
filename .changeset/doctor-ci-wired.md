---
default: patch
---

Fixes BUG-91: `doctor`'s `ci-wired` check scans every workflow rather than reading
`.github/workflows/ci.yml` by name, so a repository whose invocation lives in a differently named
workflow is no longer reported as having an unwired checker. Its `required_fields` warning also stops
claiming header rules will never fire — they do.
