# 0005 — A fifth crate, `urzua-io`, resolves the purity contradiction

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-04
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —

## Context

`urzua-core`'s own doc comment claims purity: "no I/O, no filesystem, no process. Callers read
files and hand in content." But nothing before this ADR names where discovery — walking the
filesystem, asking git which files are tracked, reading them off disk — is supposed to live. Left
unresolved, the natural failure mode is that discovery gets added to `urzua-core` the first time
someone needs it, quietly falsifying the crate's own doc comment the moment it happens, with
nothing to catch the contradiction until the mechanical purity check in 2.7 exists and someone
actually runs it against the violation.

Four crates exist today (`urzua-core`, `urzua-id`, `urzua-agdr`, `urzua-cli`) at roughly 250 lines
total. That is not enough surface to justify a boundary on its own — but the purity claim already
*is* a boundary, stated in prose without a place to put the code that would violate it if merged
in naively.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Put discovery in `urzua-core` | Fewest crates | Falsifies the crate's own purity doc comment the moment it's added |
| Put discovery directly in `urzua-cli` | Also fewest crates | `urzua-cli` becomes untestable without a real filesystem/git checkout; no reuse if a future consumer (an editor plugin, a service) needs discovery without the CLI |
| **New crate `urzua-io`** *(chosen)* | Names the boundary explicitly; `urzua-core` stays mechanically checkable as pure; discovery is unit-testable against fixture trees independent of the CLI | One more crate to version and depend on |

## Decision

In the context of a purity claim that has no enforcement mechanism and no home for the code that
would violate it, facing the certainty that discovery logic has to live somewhere before `urzua
check` can run at all, we decided to add **`urzua-io`**: filesystem discovery, git interrogation
(`git ls-files`, staged files), and file reading. `urzua-core` stays pure — schema, parsing,
validation rules, all operating on content handed to them, never reading a path themselves.
`urzua-cli` composes `urzua-io` (to gather content) with `urzua-core` (to validate it) and stays
thin.

This directly resolves the contradiction: `urzua-core`'s purity claim becomes true by construction
rather than by discipline, and 2.7's mechanical proof (assert the crate's dependency graph contains
no I/O-capable dependency) has something meaningful to check.

## Reversibility

Cheap to reverse now, at ~250 lines and before any real discovery code exists. Expensive after
`urzua-io` accumulates real logic and `urzua-core`/`urzua-cli` both depend on its shape — moving
code between crates later is a mechanical but real refactor, not a config change.

## Consequences

- `urzua-core`'s doc comment claim becomes checkable, not just asserted.
- Discovery becomes unit-testable against fixture directory trees without touching a real git
  checkout in every test.
- One more crate in the workspace, one more `Cargo.toml` to maintain — real but small at this
  scale.
- `urzua-id` and `urzua-agdr` are not touched by this decision; they remain modules-that-could-be-
  crates until they earn a boundary the same way `urzua-io` just did.

## References

- `rust/crates/urzua-core/src/lib.rs` — the purity doc comment this ADR makes true by construction.
- The mechanical purity proof (a `cargo metadata`-based dependency-graph check, with a
  planted-violation test) that verifies this boundary holds once built.
