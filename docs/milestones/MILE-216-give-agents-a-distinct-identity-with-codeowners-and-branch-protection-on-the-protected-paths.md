---
Stable-Id: 01M6XEAVA5FQ2CGSHBQMHZ3NK9
Status: Planned
Phase: '1'
Track: accountability-identity
Implements: ADR-89
Depends-on: COND-6
Blocked-on: the `main` ruleset requiring `ci` (COND-6), which agent-identity branch protection extends
---
# 216 — Give agents a distinct identity, with CODEOWNERS and branch protection on the protected paths

## What

Give agents an identity distinct from the maintainer's (a separate GitHub App or bot account with a fine-grained token), so `pr-author` can tell an agent's pull request from the maintainer's, and add `CODEOWNERS` on the protected paths with branch protection requiring the code owner. This is the future option `ADR-89` section 4 names and leaves undecided; this milestone tracks it so it is not an ownerless deferral.

## Why

`ADR-89` section 4 lists "an agent acting as the maintainer" and "a pull request that changes its own workflow" among the limits of the harness: CI cannot tell an agent from the maintainer, and only `CODEOWNERS` and branch protection, which are host settings, can stop a pull request weakening the gate it is judged by. Until a distinct identity and `CODEOWNERS` exist, the accepted risk stays open and the maintainer's review of each pull request is the only control. Whether and when to adopt it is a decision for the maintainer (an ADR when taken); this record holds the work in the roadmap meanwhile.

## Done means

- A decision record states whether agents get a distinct identity, which kind (GitHub App or bot account), and its token scope.
- An agent's pull requests are authored by that identity, and an `authority` gate with `pr-author` evidence fails an acceptance move made by it.
- A `CODEOWNERS` file names the maintainer for the protected paths declared in `.urzua/config.yaml`, and branch protection requires the code owner on them.
- The `ADR-89` section 4 residual risks "an agent acting as the maintainer" and "a pull request that changes its own workflow" are amended to say what the identity and `CODEOWNERS` now stop and what they do not.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `urzua check` reports 0 unwaived findings.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-08 | Initial record, `Status: Planned`. **Why:** `ADR-89` section 4 names a distinct agent identity plus `CODEOWNERS` and branch protection as a future option, and nothing tracked it. Maintainer-driven round-4 review. `Depends-on: COND-6` because branch protection extends the `main` ruleset that condition tracks. | **structural** |
