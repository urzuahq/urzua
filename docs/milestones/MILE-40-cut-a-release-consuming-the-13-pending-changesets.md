# 40 — Cut a release consuming the 13 pending changesets

> Status: Planned
> Stable-Id: 01M1YKX8WYPDEW5D4M1T1RHVB6
> Phase: 1
> Track: release-process
> Implements: ADR-29

## What

Trigger the existing `prepare-release.yml` `workflow_dispatch` to consume all 13 pending
`.changeset/*.md` fragments, bump `rust/Cargo.toml`'s version, and compile the real
`CHANGELOG.md` entries for the work shipped since `v0.1.0` (2026-09-05): `fix`, `audit`, `migrate
ids`, `explain`/`graph`, drift detection, header-shape rules, identity verification, the filename
type-prefix + no-padding rename, and the `mile` prefix change.

## Why

`CHANGELOG.md`'s existing `0.1.0` entry says `new`, `audit`, `migrate`, `export`, and `import` all
"exit 2 with not implemented yet" -- false today, since README's own Status table lists all but
`export`/`import` as real. Anyone reading `CHANGELOG.md` right now gets a wrong picture of what the
tool currently does. This is a maintainer-triggered act (ADR-29) -- tracked here rather than run
unilaterally.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
