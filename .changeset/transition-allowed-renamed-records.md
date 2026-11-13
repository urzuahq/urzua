---
default: patch
closes: [BUG-191]
---

`status.transition-allowed` now follows a renamed record. A record renamed or moved in the change under
`--base` is matched to its copy at the merge base by `Stable-Id` and judged against the status it had
there. Before, a rename read as a new record: renaming an `Accepted` decision gave a false error, and
renaming one while resetting its status to an initial one passed. A record with no `Stable-Id` still
reads as new.
