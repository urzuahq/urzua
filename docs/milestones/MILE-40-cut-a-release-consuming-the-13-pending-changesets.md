---
Status: Done
Stable-Id: 01M1YKX8WYPDEW5D4M1T1RHVB6
Phase: '1'
Track: release-process
Implements: ADR-45
Blocked-on: —
Realized-by: code:.github/workflows/publish-release.yml
---
# 40 — Cut a release consuming the pending changesets

## What

_Original plan, written while `ADR-29`'s manual `workflow_dispatch` release was in force. `ADR-45` has since replaced it with a reviewable release PR; this milestone now records that as its `Implements`._

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
> | 2026-11-02 | `Implements` repointed from `ADR-29` to `ADR-45`. **Why:** `ADR-29` is Superseded and `ADR-45` is the decision now in force for this work; `pointer.target-status` flagged the stale target. | **structural** |
> | 2026-09-16 | `Planned` -> `Done`. **Why:** done twice -- `v0.2.0` consumed twenty accumulated fragments into a 14,380-character changelog, and `v0.2.1` consumed one. The stale `0.1.0` changelog entry this milestone was filed against is no longer the top of the file. Cut through `ADR-45`'s reviewable release PR rather than `ADR-29`'s `workflow_dispatch`, which no longer exists. | **substantive** |
> | 2026-09-07 | Dropped the hardcoded "13" from the title (now "the pending changesets"); noted the count is 14 as of this date in the body instead. **Why:** found live during backlog triage -- the count had already drifted from 13 to 14 with nothing catching it, and hardcoding a transient number into a permanent title guarantees it drifts again. | **structural** |
> | 2026-11-02 | Labelled the body as the original `ADR-29`-era plan. **Why:** review found the body still described a release flow `ADR-45` replaced. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-177`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
