# 0006 — Shell out to `git`, rather than link `libgit2`

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —

## Context

`urzua-io` ([ADR-0005](0005-a-fifth-crate-for-io-resolves-the-purity-contradiction.md)) needs to
ask git which files are tracked and which are staged — `check` must scope discovery to what git
actually tracks, never a raw filesystem walk, or an untracked scratch file fails every run.

Two ways to talk to git from Rust: link `libgit2` (via the `git2` crate), or shell out to the
user's own `git` binary and parse its output.

The workspace's release profile (`opt-level = "z"`, `lto = true`, `codegen-units = 1`, `strip =
true`) already optimizes for startup latency and binary size over compile time — [ADR-0001](0001-rust-as-implementation-language.md)
chose Rust partly because the CLI runs in the commit hot path, where every invocation's startup
cost is paid repeatedly. Linking a C library the size of libgit2 fights that directly: larger
binary, slower cold start, and a build dependency on a C toolchain being present wherever `urzua`
is compiled from source.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Shell out to `git`** *(chosen)* | No C toolchain dependency; smaller binary; git's own CLI is stable and well-documented; matches the every invocation being tiny where `git` itself is going to be installed anyway (this is a decision-record tool for git repos) | Parsing subprocess output is less type-safe than a library API; a shell-out has process-spawn overhead per call |
| `git2` (libgit2 bindings) | Type-safe API; no subprocess spawn overhead; works even if the `git` binary isn't on `PATH` | Links a C library, fighting the release profile's own optimization target; larger binary; needs a C toolchain to build from source |

## Decision

In the context of a CLI whose own build profile is tuned for startup speed and binary size, facing
the choice between a type-safe library binding and a subprocess call for git interrogation, we
decided to **shell out to `git`**, accepting the cost of parsing subprocess output and paying a
process-spawn per call, to keep `urzua`'s own binary small and its build free of a C-toolchain
dependency.

Concretely: `urzua-io` shells out for `git ls-files` (tracked files) and `git diff --name-only
--cached` (staged files), scoping discovery to their union intersected with configured record
directories.

## Reversibility

Cheap to reverse — `urzua-io`'s discovery functions have a narrow interface (return a list of
paths); swapping the implementation from subprocess calls to `git2` bindings later is an internal
change that doesn't touch `urzua-core` or `urzua-cli` at all.

## Consequences

- `urzua` requires `git` on `PATH` at runtime. This is not a new requirement in practice — it's a
  tool for repos that are already git repos — but it's worth stating rather than assuming.
- Subprocess output parsing needs its own error handling: a non-zero exit, malformed output, or a
  missing `git` binary must all produce a reportable finding, never a silent empty result (the
  same no-silent-no-op principle discovery already has to honor).
- A non-git fallback (if ever needed) is allowed but must be *reported* in the output's scope
  information, never silent.

## References

- [ADR-0005](0005-a-fifth-crate-for-io-resolves-the-purity-contradiction.md) — the crate this
  decision lives inside.
- [ADR-0001](0001-rust-as-implementation-language.md) — the commit-hot-path startup argument this
  decision protects.
