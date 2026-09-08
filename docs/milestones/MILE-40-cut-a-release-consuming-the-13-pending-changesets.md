# 40 — Cut a release consuming the pending changesets

> Status: Planned
> Stable-Id: 01M1YKX8WYPDEW5D4M1T1RHVB6
> Phase: 1
> Track: release-process
> Implements: ADR-29
> Blocked-on: —

## What

Trigger the existing `prepare-release.yml` `workflow_dispatch` to consume all pending
`.changeset/*.md` fragments (14 as of 2026-09-07, up from 13 when this milestone was first written
-- the exact count drifts as work ships, and isn't restated as a fixed number in the title to avoid
that drift making the title itself stale), bump `rust/Cargo.toml`'s version, and compile the real
`CHANGELOG.md` entries for the work shipped since `v0.1.0` (2026-09-05): `fix`, `audit`, `migrate
ids`, `explain`/`graph`, drift detection, header-shape rules, identity verification, the filename
type-prefix + no-padding rename, and the `mile` prefix change.

## Why

`CHANGELOG.md`'s existing `0.1.0` entry says `new`, `audit`, `migrate`, `export`, and `import` all
"exit 2 with not implemented yet" -- false today, since README's own Status table lists all but
`export`/`import` as real. Anyone reading `CHANGELOG.md` right now gets a wrong picture of what the
tool currently does. This is a maintainer-triggered act (ADR-29) -- tracked here rather than run
unilaterally.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Dropped the hardcoded "13" from the title (now "the pending changesets"); noted the count is 14 as of this date in the body instead. **Why:** found live during backlog triage -- the count had already drifted from 13 to 14 with nothing catching it, and hardcoding a transient number into a permanent title guarantees it drifts again. | **structural** |
