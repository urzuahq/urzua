---
default: minor
closes: []
---

Adds `urzua check --base <ref>`: report findings, and `files_examined`, for the records that changed
since the merge base of `<ref>` and `HEAD` (staged and unstaged edits included) and for the records a
change reaches: any record that names a changed or deleted record, or cites changed or deleted
`Realized-by` evidence. Intended for a pull-request gate over a large corpus, where deleting a target
still reports the record that points at it. References resolve against the whole tracked corpus, so an
unchanged target is never reported missing, and a change to the configuration file examines every
record.

The report says what was asked for: `scope.source` is `git-diff`, `scope.base` names the ref
(`null` on every other run), and `scope.changed_paths`, `scope.dependents` and
`scope.whole_corpus_reason` account for the examined set. A diff with no changed record reports `ok`
with `files_examined: 0` and a notice, an unknown ref or one with no merge base exits 2, and `--base`
combined with a named file git does not track is refused rather than silently skipping the file.
