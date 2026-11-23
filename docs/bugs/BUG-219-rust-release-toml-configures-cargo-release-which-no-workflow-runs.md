---
Stable-Id: 01M85B6Z3JYBW6E6DXZ3FQBNJ2
Status: Open
Found-in: 'reviewing the release flow against the workflows: nothing that releases reads rust/release.toml'
Realized-by:
---
# 219 — rust/release.toml configures cargo-release, which no workflow runs

## What was wrong

`rust/release.toml` holds `cargo-release` settings (`shared-version`, `tag-name`, `push`, `publish`,
`verify`, `pre-release-commit-message`, `tag-message`). Releases are cut by `knope`
(`.github/workflows/prepare-release.yml`, `.github/workflows/publish-release.yml`, `knope.toml`), and
nothing invokes `cargo-release`: a search of `.github/`, `Makefile`, `scripts/`, `knope.toml` and `docs/`
finds the file named only in `ADR-13` and its index entry. The file reads as live release configuration and
is not. `ADR-13` called it load-bearing, which `ADR-13`'s amendment of 2026-11-23 corrects.

Checked, not inferred: with the pinned `knope` 0.23.0, `knope prepare-release --dry-run` printed the same
output with the file as committed, with its `tag-name` changed to a different value, and (for
`knope release --dry-run`) with the file removed. `cargo-release` is not installed on the review host and
no step installs it.

The suspicion that `knope.toml`'s `Release` step carries `cargo-release`-style keys is refuted:
`knope.toml` ends at `type = "Release"` with no keys. The keys are in `rust/release.toml` only.

## Why nothing caught it

`ADR-13` chose `cargo-release` on 2026-09-05, `ADR-29` and `ADR-45` replaced it with `knope`, and neither
named the file as removed, so no record said it had become dead. Nothing checks that a file named as
load-bearing is read by anything. `AGENTS.md` treats such a leftover as a finding to file, not to delete
unseen, so this record files it and the file stays until a decision on the record removes it.

## References

- `rust/release.toml`, `knope.toml`, `docs/adr/ADR-13-release-and-distribution.md` (amendment of 2026-11-23).
- `ADR-45`, which carries `knope` forward.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
