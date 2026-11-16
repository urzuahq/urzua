---
Stable-Id: 01M7K8KP682VVSMWYBEV0SMHXN
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 314 — The post-command report does not go silent when its cache is missing

## What

When the post-command step has no baseline to compare with, have it judge the commits since the merge base with the base ref, and say that it did.

## Why

On 2026-11-16 a review reproduced, with the released binary, `rm -rf .urzua/cache` followed by an edit and commit of a protected path in one command: the verdict was `allow` and nothing was printed. Session-start after the wipe reseeds from the tree as it then is, so a change made in between is adopted unreported, and it does not say it reseeded. The rule for this report is that a gated change is never silent; a cache the agent can delete must not be what that depends on.

## Done means

- With no baseline present, a command that changes a protected path or makes a gated status move is reported at error, shown by a test that deletes the cache in the same command.
- With no baseline and no base ref, the report says it could not judge and names that state; it is not `allow`, shown by a test.
- Session-start that finds no baseline says that it seeded one and that changes before it were not judged, shown by a test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-16 by an independent review of a pull request; the maintainer said that day that friction and gaps met while using the tool are improvement points to file and fix. | **structural** |
