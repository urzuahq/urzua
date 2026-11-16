---
default: patch
closes: []
---

A release now ships `scripts/install.sh`, a one-line installer for x86_64 Linux and Apple Silicon macOS that verifies the archive (its build provenance attestation through `gh`, or a sha256 you supply) before it installs anything, and checks that the installed binary reports exactly the requested version. `docs/reference/install.md` leads with it and gives the sha256 of the `v0.4.4` archives.
