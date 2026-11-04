---
Stable-Id: 01M6K4FMKWWPFXAT8N7Z0SG99H
Status: Planned
Phase: '0'
Track: governance-process
Implements: RFC-7, ADR-71
Blocked-on: —
---
# 180 — A reusable CI integration installs a pinned urzua release, runs the gate and reports findings on the pull request

## What

A reusable CI integration for a repository that adopts urzua: install a pinned release binary, verify its checksum and build provenance, check that the checkout has the history drift reads, run `urzua check` and on a pull request `urzua check --base`, and report the findings on the pull request as inline annotations and one summary comment that is updated rather than stacked. The renderer reads urzua's JSON from outside the engine (stdout stays JSON, `ADR-23`), and a failed comment post never turns a clean gate red.

## Why

A real adopter environment's records job installs its tool, runs the gate with full history and a PR base, annotates findings on the diff and posts one updating drift summary. Here `checks.yml` builds from source and runs `check` and `check --base` on this repository's own tree; there is no unit another repository can reuse, so each adopter would rewrite the install, checksum, history-depth and reporting steps, the duplication urzua exists to remove. The release workflow already publishes `urzua-<target>.tar.gz` for three targets with build provenance attestation, and no Linux arm64 build is published. A shallow checkout makes drift unable to detect anything and reads as clean, which `checks.yml` guards with a comment and `MILE-159` records as a cause of an earlier false zero. It is capability parity that a real adopter corpus needs from urzua's governance, and Phase 0 is urzua reproducing that functionality in its own repository.

`RFC-27` section 3 keeps anything that needs a hosted service out of the engine; this is a consumer of the JSON contract, so the milestone's unit lives beside the engine (a composite action or a script in `.github/`), not in `urzua-core`.

## Done means

The unit installs a pinned release, fails when checksum or attestation verification fails, and fails with a stated reason when the checkout is too shallow for drift; this repository's pull-request CI runs the renderer over its own `urzua check --base` output and posts annotations and one updating comment (with a recorded-fixture test of the rendering and of the update-in-place behaviour); a job installs the previous release through the unit and runs it over this repository's corpus; whether to publish a Linux arm64 build is decided and recorded; and the capability is what this repository's own workflows run, not only a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed. **Why:** a trial run of urzua over a real adopter corpus showed that environment's records CI job and pull-request reporting have no reusable counterpart here; Phase 0 is capability parity exercised in urzua's own repository. | **substantive** |
