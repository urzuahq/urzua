---
Stable-Id: 01M6NKGPG7W1PZFAKXRF8EPBM8
Rule: status.delivery-agreement
Scope: docs/bugs/BUG-14-knope-s-preparerelease-bumps-cargo-toml-but-never-regenerates-the-workspace-cargo-lock.md
Reason: "Fixed in `knope.toml`: the `prepare-release` workflow now runs `cargo check --manifest-path rust/Cargo.toml --workspace` after `PrepareRelease` and stages the regenerated `Cargo.lock`. That step runs only when a release PR is prepared, and nothing in this repository's test suite runs the release workflow. The record states it was verified by hand, producing exactly the five workspace-member version bumps. (The step later dropped `--offline`, which needs a populated registry the release job lacks; `BUG-27`.)"
Category: docs-or-workflow-only
---
# WAIVER-0036 — BUG-14 needs no test: its fix is release workflow configuration

## What this covers

`status.delivery-agreement` reports `BUG-14`: its `Status` is `Fixed`, but no `Realized-by` evidence computes to a delivered tier. Fixed in `knope.toml`: the `prepare-release` workflow now runs `cargo check --manifest-path rust/Cargo.toml --workspace` after `PrepareRelease` and stages the regenerated `Cargo.lock`. That step runs only when a release PR is prepared, and nothing in this repository's test suite runs the release workflow. The record states it was verified by hand, producing exactly the five workspace-member version bumps. (The step later dropped `--offline`, which needs a populated registry the release job lacks; `BUG-27`.)

The waiver has no `Expires`: the cause is a permanent fact about the record, and the `permanent` declaration in `.urzua/config.yaml` allows it under `docs-or-workflow-only`. It is reported by `waiver.matches-nothing` if the record gains evidence and the finding stops.

## References

- `docs/bugs/BUG-14-knope-s-preparerelease-bumps-cargo-toml-but-never-regenerates-the-workspace-cargo-lock.md` — the record whose status this waives.
- `ADR-84` — the rule; `ADR-86` — permanent waivers under a declared category; `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed with the `ADR-84` build (`status.delivery-agreement` on bugs). **Why:** the record is `Fixed` with a reason no test can cover, stated in `Reason`. | **substantive** |
