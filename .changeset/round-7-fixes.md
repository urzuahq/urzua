---
default: patch
---

Fixes BUG-67: a path scope selects findings by the requested prefix, so a finding about a file git
does not track is no longer dropped and `check .` agrees with `check`.

Fixes BUG-68: a `claim_paths` entry naming a file is rejected at load rather than scanned as an empty
directory.

Fixes BUG-69: the `claim_paths` walk does not descend into symlinks.

Fixes BUG-70: `init` withholds the identity rules only when no proposed type has a prefix, not when
any one lacks it.

Fixes BUG-71: `type.record-outside-declared-dir` decides ownership from the path, so a staged
deletion or an unreadable file directly inside a declared dir is not reported as unowned.

Fixes BUG-72: an empty type prefix is no longer read as a reference.

Fixes BUG-59: `narrative-field.stale` reads a declared `terminal_statuses`, required like
`pointer.target-status`'s `not_in`. `is_terminal_status` is deleted.
