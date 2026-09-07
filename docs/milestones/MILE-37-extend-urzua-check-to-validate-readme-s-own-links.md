# 37 — Extend urzua check to validate README's own links

> Status: Planned
> Stable-Id: 01M1YJGWJP6SWNNWQ0Z6AX7YQ9
> Phase: 1
> Track: roadmap-tracking

## What

Teach `urzua check` to scan markdown links in root-level docs (`README.md`, `CONTRIBUTING.md`,
`CHANGELOG.md`) against the real corpus, resolving them the same way `pointer.resolution` already
resolves `Implements:`/`Derives-from:` -- so a renamed, renumbered, or deleted ADR/RFC/spec that a
root doc references by inline citation or markdown link is caught, not silently stale.

## Why

Record-to-record cross-references (`Implements: ADR-4`) are already validated by
`pointer.resolution`. The ~15 inline ADR/RFC references in README.md are not -- they're prose in a
file `urzua` doesn't treat as a record at all, so a rename here goes uncaught exactly the way a
record's own dangling pointer used to before this project built a check for it. Found live while
reviewing README for staleness after the ADR-36 filename-convention rename and the `MILE` prefix
change, both of which already required a manual sweep of README's own citations.

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
