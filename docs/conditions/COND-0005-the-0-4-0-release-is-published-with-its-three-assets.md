---
Stable-Id: 01M6SSWKTFSVWBJYC4Q540X52K
Status: Open
---
# COND-0005 — The 0.4.0 release is published with its three assets

## Signal

Met when a `v0.4.0` tag is published on `urzuahq/urzua` as a release with three assets (the release binary archive, its checksum file and its build provenance attestation). The maintainer reports it in `Met-by`, after, in order, setting the release App secrets (`RELEASE_APP_CLIENT_ID`, `RELEASE_APP_PRIVATE_KEY`), adding a ruleset on `main`, and cutting the release. The release page listing three assets is the evidence; a tag alone, a draft or a release with fewer assets is not met.

## Why this is a condition

`v0.4.0` is not cut and the gate action and README still pin `v0.3.0`, while main is hundreds of commits ahead and the pinned release cannot read the current `.urzua/config.yaml` (adopter H1). `prepare-release` fails on every push to main because the release App secrets are unset, and main has no ruleset. All three steps are the maintainer's.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Initial record. **Why:** the release steps deferred in Phase 0 hardening are the maintainer's and no record tracked them. | **structural** |
> | 2026-11-07 | The first secret is named `RELEASE_APP_CLIENT_ID`, not `RELEASE_APP_ID`. **Why:** `prepare-release.yml` reads the client id since #404; the steps are in `docs/reference/release-app-setup.md`. | **structural** |
