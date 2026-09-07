---
default: minor
---

# `fix --apply`/`new` now trust a verified `gh` login over `--by`

Identity resolution for `fix --apply` and `new` (RFC-0002/ADR-0031) now checks `gh api user` first,
then `git config user.name`, and only falls back to an explicit `--by` value when neither verified
source is available. Previously `--by` won outright, which meant a typed name could silently
override an authenticated `gh` session. If you were relying on `--by` overriding your `gh` login,
that no longer happens -- unset `GH_TOKEN`/log out of `gh` in that shell, or don't authenticate `gh`
in that environment. `gh api user` is also now bounded to 5 seconds rather than hanging indefinitely
if `gh` doesn't respond.
