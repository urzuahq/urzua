# 0001 — Rust as the implementation language

> Status: Accepted
> Embodiment: Not started
> Date: 2026-07-29
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —

## Context

Rust was named as the target language in the originating work but never justified in writing — the
README explicitly flagged this as an assumption not to inherit uncritically. This ADR closes that
gap before any code exists, which is the cheapest moment to be wrong.

The constraint that actually drives the choice comes from the founding premise: governance
machinery like this tends to get reinvented independently, per engineering org, per stack.
**"Build it once instead of many times" is only true if one binary installs identically into every
host repo regardless of that repo's stack.** A tool that requires the host project to adopt a
Python or Node runtime has reintroduced the per-repo cost it exists to remove — a real deal-breaker
for any codebase that has deliberately kept its own dependency footprint minimal.

The second constraint is where the tool runs: pre-commit and pre-push hooks, CI, and eventually
every agent action. That's the hot path, invoked constantly, and the relevant cost is **process
startup**, not throughput. Evidence from the source work is direct on this: a governance check that
is slow or awkward in the hook path gets disabled, and a disabled check is indistinguishable from no
check — the same failure class as a linter that reports zero errors because it silently examined
nothing.

Notably, raw compute is *not* the justification. Validating a few hundred markdown files is not
compute-bound, and "Rust is fast" would be a post-hoc rationalization rather than a reason.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Rust** | Single static binary, no host runtime; negligible startup in hook path; distributable via Homebrew/cargo/GitHub releases; strong correctness properties for a tool that gates commits | Slowest to first dogfood; smallest contributor pool; no reuse of the existing TypeScript implementation |
| **TypeScript/Node** | Fastest path to dogfooding — one target codebase is already a TS monorepo with a working linter to port; largest contributor pool; natural fit for the agent-harness ecosystem | Requires Node in every host repo, breaking the cross-stack claim; interpreter startup on every hook invocation; `node_modules` in someone else's repo is a real adoption objection |
| **Python** | Also has a working implementation to port; ubiquitous in one target codebase | Worst distribution story of the three (virtualenvs, version skew); startup cost; same cross-stack problem as Node |
| **Go** | Single binary and fast startup — matches Rust on both primary constraints; faster to write than Rust | No existing implementation to port either; weaker type-level guarantees for the schema/validation core; largely a wash against Rust on the properties that decided this |

## Decision

In the context of a governance tool that must install into heterogeneous repos it doesn't own and
run in the commit hot path, facing a founding claim that collapses if the tool imposes a runtime on
its host, we decided to implement in **Rust**, to get single-binary distribution and negligible
startup cost, accepting slower time-to-first-dogfood and abandoning reuse of the existing
TypeScript and Python implementations.

Go was the closest alternative and would have been defensible. Rust wins on type-level modelling of
the record schema and validation states, which is the core of the product rather than incidental.

## Reversibility

**Moderately reversible, and cheaply so while v0 is small.** The user-facing contract is a CLI —
commands, exit codes, config format, and record format — none of which are language-bound. A
rewrite would be expensive in time but wouldn't strand users or data, since the records are plain
files.

Reversibility drops sharply once there are downstream integrations (editor plugins, agent-harness
hooks, a library API). The window for changing this cheaply is roughly "before v0 ships."

## Consequences

- Time to first dogfood is longer than the TypeScript path. Accepted deliberately; distribution is
  the strategic property and dogfooding speed is not.
- The existing TS and Python linters become **specifications rather than starting points** — port
  the logic and the documented bug history, not the code. Their bug histories are the acceptance
  test corpus (see the v0 spec).
- Distribution via Homebrew, `cargo install`, and GitHub release binaries. No host-repo runtime, no
  lockfile, nothing added to the host's dependency tree.
- Contributor pool is narrower, which matters more if this is ever open-sourced than it does now.
- CI needs a cross-compilation matrix (macOS arm64/x86, Linux) from day one — a real setup cost
  incurred before the first useful commit.
- Schema and validation states get modelled in the type system, which is the main reason to prefer
  this over Go.

## References

- `README.md` — flagged Rust as named but unjustified
