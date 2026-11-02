---
Stable-Id: 01M6CXZWVJHC0QV4P001H2KS0V
Status: Planned
Phase: '0'
Track: schema-governance
---
# 123 — check scope source git-diff: examine only what changed against a base ref

## What

Build the `git-diff` value of `scope.source` in `check`'s report: report findings and `files_examined` only for
the records that changed against a base ref, and report the base in the output. It is a reporting
scope, not a reading scope: reference resolution still indexes the full corpus, so an unchanged target
is never reported missing, and rules that need the changed paths themselves (`MILE-124`) receive them
alongside the full index. `SPEC-2` lists it in its scope table as
`git-diff | changed against a base ref | the ref | no -- the mode is not built`.

## Why

A repository of several hundred records should not treat every run as a full sweep in a pull
request gate, and an adopter replacing a linter outright will not accept a slower or noisier one.
`SPEC-2` reserves the value in the report's contract for future use; today `ScopeSource` emits only
`tracked-sweep` and `argv`, and consumers must not branch on a value no run produces. Per no-silent-no-op, the report must say what base it used and what it examined,
not only what it found.

## References

- `SPEC-2` -- the `scope.source` table that records the mode as unbuilt.
- `MILE-102` -- the adopter replacement this serves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `SPEC-2` documents `git-diff` scope as not built; a pull-request gate over a large corpus needs it, so it is now planned. | **substantive** |
