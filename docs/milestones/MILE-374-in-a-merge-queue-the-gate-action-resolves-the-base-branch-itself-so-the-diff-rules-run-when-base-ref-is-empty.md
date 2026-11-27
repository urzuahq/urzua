---
Stable-Id: 01M8DXZSHNS4VXGZJB0CGVM9HX
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: —
---
# 374 — In a merge queue the gate action resolves the base branch itself, so the diff rules run when base-ref is empty

## What

When the `base-ref` input of `.github/actions/urzua-gate` is empty and the event is `merge_group`, the action takes the base from `github.event.merge_group.base_ref`, with `refs/heads/` stripped, and runs `urzua check --base origin/<base>` as it does for a pull request. A caller that passes `${{ github.base_ref }}`, as the action's own usage comment shows, gets the diff rules in a merge queue without changing its workflow.

## Why

On a `merge_group` event `github.base_ref` is empty. The action passes `inputs.base-ref` straight to `check-history.sh` and to the `run urzua check` step, which runs the base check only `if [ -n "$BASE_REF" ]` (`action.yml`). So a caller that passes `github.base_ref` gets only the whole-tree check in a merge queue. The diff rules, drift included, are skipped, and the result doesn't say so. This repository's own `checks.yml` already avoids that: its base-branch step falls back to `QUEUE_BASE: ${{ github.event.merge_group.base_ref }}` and strips `refs/heads/`. A caller has to copy that step to get the same result. No caller runs a merge queue today, so the gap is latent. The org orchestrator raised it on 2026-11-27, and it is low priority.

## Done means

- `.github/actions/urzua-gate/action.yml`: with `base-ref` empty on a `merge_group` event, the action runs the base check against `origin/<merge_group base, refs/heads/ stripped>`. A non-empty `base-ref` still wins.
- `.github/actions/urzua-gate/test.sh` covers the resolution: an empty input on a `merge_group` event resolves to the queue's base, an explicit input is kept, and an empty input on any other event stays empty. The test is seen failing before the change.
- The `base-ref` input's description and the usage comment at the top of `action.yml` say the action resolves the base itself in a merge queue.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
