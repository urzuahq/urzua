---
default: minor
---

# `fix --apply`/`new` now prefer an authenticated `gh` session over `--by`

Identity resolution for `fix --apply` and `new` (RFC-2/ADR-31) now checks `gh api user` first,
then `git config user.name`, and only falls back to an explicit `--by` value when neither source is
available. Previously `--by` won outright, which meant a typed name could silently override an
authenticated `gh` session. If you were relying on `--by` overriding your `gh` login, that no longer
happens -- unset `GH_TOKEN`/log out of `gh` in that shell, or don't authenticate `gh` in that
environment. `gh api user` is also now bounded to 5 seconds rather than hanging indefinitely if `gh`
doesn't respond.

None of these sources is a cryptographic attestation -- each is just harder to spoof by accident
than the one below it (RFC-2's own stated goal is raising the cost of a careless rubber-stamp, not
achieving unforgeable proof). Treat the resulting `Author`/`Tool-authored (by ...)` value as
attribution, not verification of who actually reviewed the change.
