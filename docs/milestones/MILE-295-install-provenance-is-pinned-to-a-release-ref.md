---
Stable-Id: 01M7HW24Z5TFZX8M6G1DNFD87V
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 295 — Install provenance is pinned to a release ref

## What

Tighten the build provenance check the installer, the gate action and the documented manual steps make, so an attestation from a modified copy of the release workflow on another ref does not pass.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. The review of the install script found the provenance check pinned to the repository and the workflow file but not to a ref: someone with push access to a branch could obtain an attestation from an edited copy of the workflow.

## Done means

- The installer, the gate action's installer and the manual steps verify that the attested build came from a release tag's commit, shown by a test with an attestation from another ref.
- The install page says what the provenance check proves and what it does not.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
