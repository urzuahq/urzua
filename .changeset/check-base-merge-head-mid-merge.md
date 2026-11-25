---
default: patch
closes: [BUG-235]
---

`urzua check --base <ref>` run in the middle of a merge now takes the merge base against the pending merge commit (`HEAD` and every line of `MERGE_HEAD`), so `--base MERGE_HEAD` reports what the committed merge will report against the branch merged in. It used the merge base with the old tip, which charged every commit that came in to the merge.
