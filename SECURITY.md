# Security policy

Urzua is a local CLI: it reads files you point it at and writes to stdout/stderr and paths you
configure. It makes no network calls and has no server component, which limits the attack surface
considerably — but a validator that parses untrusted input (arbitrary repository content) is still
worth reporting issues against.

## Reporting a vulnerability

Please do not open a public GitHub issue for a security report. Instead, use GitHub's private
[security advisory](../../security/advisories/new) feature for this repository, or contact the
maintainers directly if that isn't available to you.

Include what you'd include in any good bug report: the affected version/commit, a reproduction
case, and the impact as you understand it.

## Scope

In scope: memory safety, panics on untrusted input that should instead produce a diagnostic,
path traversal or arbitrary file write outside the target corpus, and anything that would let a
malicious record file cause `urzua` to do something other than report on it.

Out of scope: the correctness of a specific lint rule (that's a bug report, not a security report)
and denial-of-service via a maliciously large input (file a normal issue).
