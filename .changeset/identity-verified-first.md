---
default: minor
---

# `fix --apply`/`new` now prefer an authenticated `gh` session over `--by`; `new` gains a `--by` flag

Identity resolution for `fix --apply` and `new` (RFC-2/ADR-31) now checks `gh api user` first, then
falls back to an explicit `--by` value, and only then to `git config user.name` if neither is
available. Previously `--by` won outright, which meant a typed name could silently override an
authenticated `gh` session -- that no longer happens. If a `--by` is given and it differs from an
authenticated `gh` login, `gh`'s login still wins; the divergence is surfaced as a `warnings` entry
in the command's JSON output rather than the value being silently discarded. `git config user.name`
is equally unsigned local input as `--by`, so it no longer outranks it (ADR-31's amendment).
`urzua new` also gains the `--by` flag `fix --apply` already had. `gh api user` is also now bounded
to 5 seconds rather than hanging indefinitely if `gh` doesn't respond.

None of these sources is a cryptographic attestation -- each is just harder to spoof by accident
than the one below it (RFC-2's own stated goal is raising the cost of a careless rubber-stamp, not
achieving unforgeable proof). Treat the resulting `Author`/`Tool-authored (by ...)` value as
attribution, not verification of who actually reviewed the change.
