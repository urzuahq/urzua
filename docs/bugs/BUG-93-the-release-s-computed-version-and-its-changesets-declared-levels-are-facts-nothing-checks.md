---
Stable-Id: 01M30EGKJNY72B2NMW9GH47HR2
Status: Fixed
Found-in: "Two release defects in one evening, both with the same single symptom"
Regression-test: ".github/scripts/release-invariants.test.sh"
---
# 93 — The release's computed version and its changesets' declared levels are facts nothing checks

## What was wrong

Two defects shipped within hours of each other, and neither was caught by anything:

1. **`BUG-92`** -- a release-prep version bump reached `main` through a bugfix PR. The manifest said
   `0.4.0` while the newest tag was `v0.3.0`, so `knope` computed every subsequent release from a
   version that had never been published.
2. **The changeset level.** The consolidated fragment declared `default: minor` while its first line
   read *"Breaking: the config is now `.urzua/config.yaml` at `schema_version: 2`. A `0.3.0` config
   does not load."* Pre-1.0, `knope` maps `major` to a minor bump and `minor` to a **patch** bump, so
   that fragment would have shipped a config-breaking change as `0.3.1` with **no Breaking Changes
   section in the CHANGELOG**. An adopter taking a patch upgrade would get a config that does not
   load, and nothing in the release notes would say so.

Both are facts about the release, and nothing read either. `make ci` has no reason to care what
version the release will be.

## Why nothing caught them

**They had the same single symptom**: `PR #72`'s title changing. It went `prepare release 0.4.0` ->
`0.4.1` -> `0.3.1` over two days as the inputs moved. A title changing quietly on an already-open PR
is not something anyone re-reads, and it was missed twice.

`ADR-49`'s `release-guard.sh` already exists for this family -- `BUG-48` was a changeset naming a
package `knope.toml` did not declare, silently inert, with seven accumulating behind it while
`prepare release` reported success. Its own comment states the principle: *"a guard that has never
been observed failing is indistinguishable from one that cannot fire."* Neither of these facts had a
guard at all.

## The guards

`release-invariants.sh`, wired into `make ci` beside `release-guard`:

1. **With fragments pending, the manifest version must equal the newest tag.** Nothing has been
   released since that tag, so a manifest ahead of it means a prep bump arrived early. Only checked
   while fragments pend: `publish-release` merges to `main` before `knope release` tags, so a
   legitimate window exists where the manifest leads -- and that window always has an empty changeset
   directory.
2. **A fragment whose text describes a breaking change must declare `major`.**

Seven planted cases, each asserted in both directions.

The second guard caught the live defect on its first run against `main`, before `PR #92` had merged --
an unplanted violation. It then caught **its own changeset**, which described breaking changes while
declaring none, so the pattern was narrowed from a keyword mention to a declaration
(`^**Breaking:` or `^Breaking:`, which is how a breaking fragment is actually written). That is the
same wrong-in-both-directions failure `claim.status-agreement` already documents for its closing
verbs, found here within minutes of the guard existing.

## References

- `BUG-92`, one of the two defects, and `BUG-48`, the same family one step earlier.
- `ADR-49`, the release guard this sits beside.
- `ADR-55` -- a guard that cannot fire is a check reporting success without looking.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
