---
default: patch
closes: []
---

`urzua new` reports the numbers it passed over in its JSON output instead of on stderr. The report gains
a `skipped` list, always present, with one `{number, reason}` entry for each number and reason: `open-pr
<ref>` for a number held by a pull request's head ref, `worktree <path>` for one held in a sibling
worktree, and `local` for one held by a local branch or remote-tracking ref. The `urzua: skipped N` lines
are gone. When git cannot read the other refs, `new` numbers from the working tree as before and says so in
a warning notice (subject `new.numbering`) rather than on stderr.
