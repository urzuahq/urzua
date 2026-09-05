# 0013 — Release and distribution: GitHub binaries, cargo-release, gated on CI

> Status: Accepted
> Embodiment: Not started
> Date: 2026-09-05
> Author: (project lead)
> Deciders: (project lead)
> Supersedes / Superseded-by: —
> Derives-from: SPEC-0001

## Context

SPEC-0001 requires cross-compilation "from day one" and warns that release automation must not
assume a workspace at the repo root, since release artifacts build from `rust/`. Three separate
questions needed answers before a first release could exist: how versions are assigned and tagged,
which install channel(s) v0 targets, and whether a release should actually be cut now, while the
repo is still private.

## Decision

In the context of a v0 CLI with no external users yet, facing three separable release-mechanics
questions, we decided:

**Versioning: `cargo-release`.** Adopted over a hand-rolled tag-bump `make` target for the built-in
version-bump/commit/tag sequencing, configured via `rust/release.toml` with `publish = false` and
`verify = false` — cargo-release's own crates.io-publish machinery is turned off entirely, since this
project isn't publishing there (see below).

**Distribution: GitHub release binaries only, not crates.io, not Homebrew.** Every crate carries
`publish = false` (set once at `[workspace.package]`, inherited via `publish.workspace = true`) so a
`cargo publish` accident is impossible, not just discouraged. Cross-compiled for
`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-gnu` — the release workflow
explicitly sets `working-directory: rust` throughout, honoring SPEC-0001's warning. crates.io and
Homebrew remain candidates for later, once there's real install-channel demand to justify the extra
maintenance surface (a Homebrew tap is a second repo; crates.io is a second audience to support).

**Release-gating**: the release workflow's first job queries the `ci` check's conclusion for the
exact tagged commit via the GitHub API and refuses to proceed unless it reads `success` — a tag
pushed against a red or unevaluated commit produces no release. Every release binary attempts build
provenance via `actions/attest-build-provenance`, non-blocking (`continue-on-error`) — GitHub
refuses this for a private repo without a paid plan ("Feature not available ... upgrade the billing
plan, or make this repository public"), discovered by the first real release run against this
repo's actual, still-private state. Attestation becomes real once the repo is public; until then the
step fails visibly in the run log without blocking the release.

**Cut now, while still private.** Rather than wiring the pipeline and leaving it untriggered until
public launch, `v0.1.0` is tagged now — proving the pipeline end to end while it's cheap to fix
(private, no external consumer depending on it) rather than discovering a break at the moment it
matters most.

## Reversibility

Adding crates.io/Homebrew later is additive, not a reversal — `publish = false` is easy to flip
per-crate when there's a reason to. The `cargo-release` choice is a tooling preference, cheaply
swapped for a hand-rolled script if it doesn't fit; nothing in the release *workflow* depends on
which tool produced the tag. Cutting `v0.1.0` now, on a private repo, has no real consumer to
disappoint if something about the release shape needs to change before public launch.

## Consequences

- `rust/release.toml` and `publish.workspace = true`/`publish = false` are now load-bearing —
  removing them without updating the release process would silently re-enable `cargo publish`.
- The release workflow assumes a `ci` check by that exact name exists on the tagged commit — renaming
  the CI workflow's fan-in job breaks release-gating silently unless this ADR's assumption is
  remembered.
- A public launch needs no new release-mechanics work — only visibility (making the repo public)
  changes, not the pipeline.
- Build provenance is currently a no-op in every release: the attestation step fails and is
  swallowed by `continue-on-error` on this still-private repo. This is a known, visible gap, not a
  silent one — worth re-checking (`git grep continue-on-error .github/workflows/release.yml`) once
  the repo goes public, so the non-blocking escape hatch doesn't quietly outlive its reason.

## References

- SPEC-0001 — the cross-compilation and `rust/`-not-root requirements this ADR satisfies.
- `.github/workflows/release.yml`, `rust/release.toml` — the implementation.
- `.github/workflows/ci.yml`'s `ci` job — the release-gating check this ADR depends on by name.
