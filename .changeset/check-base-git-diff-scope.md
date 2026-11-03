---
default: minor
---

Adds `urzua check --base <ref>`: report findings, and `files_examined`, for only the records that
changed since the merge base of `<ref>` and `HEAD`, including staged and unstaged edits. Intended for
a pull-request gate over a large corpus. References still resolve against the whole tracked corpus,
so an unchanged target is never reported missing.

The report says what was asked for: `scope.source` is `git-diff` and the new `scope.base` names the
ref (`scope.base` is `null` on every other run). A run where nothing changed reports `not-run`
rather than a clean pass, an unknown ref exits 2, and `--base` combined with a named file git does
not track is refused rather than silently skipping the file.
