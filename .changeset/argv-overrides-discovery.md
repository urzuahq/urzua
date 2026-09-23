---
default: minor
---

Fixes `BUG-24`: an explicitly-named `check <file>` path that git doesn't track was silently dropped
(`"status": "not-run", "files_examined": 0"`, no findings, no notice) instead of being read, exactly
as SPEC-2's Discovery contract already said it should be ("explicit paths on argv override discovery
and are used as given"). A directly-named file now overrides the tracked-only filter; a directory
argument still respects the tracked-only sweep unchanged (`ADR-6`'s "never a raw directory walk") --
only naming one file precisely bypasses it. Additive to the tracked sweep, never a replacement: the
corpus a pointer resolves against still doesn't shrink just because a path was requested (`BUG-60`).

New `ScopeSource`/`DiscoverySource::Argv` report which mode actually ran, distinct from
`tracked-sweep`.

Also adds `docs/specs/SPEC-22-rust-coding-standards.md` (Draft), surveying and backlinking this
codebase's already-in-force Rust conventions (domain-value newtypes, the declared-population pattern,
shared candidate-selection helpers, `#![forbid(unsafe_code)]`, the pure-core/impure-io split, and
more) to where each was decided or landed -- none of it was written down anywhere before.

No adopter-facing behavior change for a `check` invocation naming only tracked files or directories:
verified with the full test suite, clippy, `make ci`, and a real-corpus `check` run reporting the
same 70 findings before and after.
