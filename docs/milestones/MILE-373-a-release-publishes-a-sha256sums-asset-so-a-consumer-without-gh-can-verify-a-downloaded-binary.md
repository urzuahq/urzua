---
Stable-Id: 01M8DF7YFHNJN666M11K3KX030
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: —
---
# 373 — A release publishes a SHA256SUMS asset so a consumer without gh can verify a downloaded binary

## What

Each GitHub release carries a `SHA256SUMS` asset listing the sha256 of every archive it publishes. The asset gets a build provenance attestation like the archives, so a consumer without `gh` has a published digest to pass to `scripts/install.sh --sha256` or to check by hand.

## Why

On 2026-11-27 a consumer installing 0.4.15 by hand reported that the release has no checksum file to verify against. The org orchestrator relayed the report. Checked the same day:

- The `v0.4.15` release has two assets, `urzua-x86_64-unknown-linux-gnu.tar.gz` and `urzua-aarch64-apple-darwin.tar.gz`, and no checksum file.
- The Linux archive's build provenance attestation verifies with `gh attestation verify <archive> -R urzuahq/urzua`. It was signed by `.github/workflows/publish-release.yml` on `refs/heads/main`, so a consumer with an authenticated `gh` can verify.
- Without `gh`, `scripts/install.sh` requires `--sha256` (or `--no-verify`). The release publishes no digest to take it from, so the documented path without `gh` has no trustworthy source for the value.

Low priority: the attestation path works, and nothing else waits on this.

## Done means

- `.github/workflows/publish-release.yml` uploads a `SHA256SUMS` asset whose lines cover every archive the release publishes, and the asset gets a build provenance attestation. The workflow is a protected path, so a person applies this edit.
- `scripts/install.sh`'s usage text and the install documentation name the release's `SHA256SUMS` asset as the source for `--sha256`.
- A test checks that the publish workflow's uploaded asset list includes `SHA256SUMS` alongside every archive.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
