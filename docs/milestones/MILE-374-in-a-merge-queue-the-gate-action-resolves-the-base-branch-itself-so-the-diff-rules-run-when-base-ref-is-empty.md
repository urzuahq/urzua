---
Stable-Id: 01M8DXZSHNS4VXGZJB0CGVM9HX
Status: Done
Phase: '1'
Track: integrations
Blocked-on: —
Realized-by: code:.github/actions/urzua-gate/action.yml@sha256:e0b427682f8559d8bd8cf67d581cd7b8c609b50a1aeabc1d86f818846115c8aa, code:.github/actions/urzua-gate/resolve-base.sh@sha256:1ccdda7a0f3a2ebd1df870e4a76ea8fff46950446080e200a63335294e1f0055, test:.github/actions/urzua-gate/test.sh@sha256:38574e3fdf95e8b75f90a66b4854d7ec45cc41787f4e51fda4aed044cb8f7bae
---
# 374 — In a merge queue the gate action resolves the base branch itself, so the diff rules run when base-ref is empty

## What

When the `base-ref` input of `.github/actions/urzua-gate` is empty and the event is `merge_group`, the action takes the base from `github.event.merge_group.base_ref`, with `refs/heads/` stripped, and runs `urzua check --base origin/<base>` as it does for a pull request. A caller that passes `${{ github.base_ref }}`, as the action's own usage comment shows, gets the diff rules in a merge queue without changing its workflow.

## Why

On a `merge_group` event `github.base_ref` is empty. The action passes `inputs.base-ref` straight to `check-history.sh` and to the `run urzua check` step, which runs the base check only `if [ -n "$BASE_REF" ]` (`action.yml`). So a caller that passes `github.base_ref` gets only the whole-tree check in a merge queue. The diff rules, drift included, are skipped, and the result doesn't say so. This repository's own `checks.yml` already avoids that: its base-branch step falls back to `QUEUE_BASE: ${{ github.event.merge_group.base_ref }}` and strips `refs/heads/`. A caller has to copy that step to get the same result. No caller runs a merge queue today, so the gap is latent. The org orchestrator raised it on 2026-11-27, and it is low priority.

## Done means

- `.github/actions/urzua-gate/action.yml`: with `base-ref` empty on a `merge_group` event, the action runs the base check against `origin/<merge_group base, refs/heads/ stripped>`. A non-empty `base-ref` still wins. — evidence: code:.github/actions/urzua-gate/action.yml@sha256:e0b427682f8559d8bd8cf67d581cd7b8c609b50a1aeabc1d86f818846115c8aa, code:.github/actions/urzua-gate/resolve-base.sh@sha256:1ccdda7a0f3a2ebd1df870e4a76ea8fff46950446080e200a63335294e1f0055
- `.github/actions/urzua-gate/test.sh` covers the resolution: an empty input on a `merge_group` event resolves to the queue's base, an explicit input is kept, and an empty input on any other event stays empty. The test is seen failing before the change. — evidence: test:.github/actions/urzua-gate/test.sh@sha256:38574e3fdf95e8b75f90a66b4854d7ec45cc41787f4e51fda4aed044cb8f7bae
- The `base-ref` input's description and the usage comment at the top of `action.yml` say the action resolves the base itself in a merge queue. — evidence: code:.github/actions/urzua-gate/action.yml@sha256:e0b427682f8559d8bd8cf67d581cd7b8c609b50a1aeabc1d86f818846115c8aa

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-28 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .github/actions/urzua-gate/action.yml@sha256:e0b427682f85, .github/actions/urzua-gate/resolve-base.sh@sha256:1ccdda7a0f3a, .github/actions/urzua-gate/test.sh@sha256:38574e3fdf95 Reason: Pinned the action, its base-resolution script and the test that covers it, which deliver this milestone (PR #659) **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-28 | Delivered by PR #659: the gate action resolves the base itself on a `merge_group` event when `base-ref` is empty (`resolve-base.sh`), a set `base-ref` wins, and `test.sh` covers the three cases; the `base-ref` description and the usage comment say so. All three Done means items are met, so Status is Done. `Realized-by` names the action, the script and the test. | **structural** |
> | 2026-11-28 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .github/actions/urzua-gate/action.yml@sha256:e0b427682f85, .github/actions/urzua-gate/resolve-base.sh@sha256:1ccdda7a0f3a, .github/actions/urzua-gate/test.sh@sha256:38574e3fdf95 Reason: Pinned the evidence clauses of the Done means items to the action, its script and the test (PR #659) **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
