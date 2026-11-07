---
default: minor
closes: [MILE-206]
---

Adds `discovery.include_untracked`, an opt-in configuration option that makes `urzua check` read the record files you have written and not yet staged. With `discovery: {include_untracked: true}`, discovery includes untracked, non-ignored files directly under a declared record directory (git's own ignore rules decide), so a malformed draft is found before `git add`. Under `--base`, such a record counts as added and the diff rules judge it like a staged addition. Embodiment drift cannot judge a record with no git history; an info notice, `discovery.untracked-not-drift-judged`, counts the records it skipped for that reason. The report's `scope` gains `include_untracked` and `untracked_included`.

Without the option, nothing changes in what is examined, but a plain `check` no longer stays silent about drafts: an info notice, `discovery.untracked-skipped`, names the untracked records under declared directories that it did not examine (at most 20 paths and the count). It does not change the exit code. An unknown key under `discovery`, or a value that is not a boolean, is a load error.
