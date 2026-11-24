---
Stable-Id: 01M85Q8VABJB05ZVFV262QP3PV
Status: Open
---
# COND-0007 — the published agent CLI contract tags a release candidate carrying the conformance runner

## Signal

Met when the published agent CLI contract tags a release candidate whose tree contains the conformance runner (`scripts/conform.py`) and the schemas it checks. The runner is in none of `v1.0.0`, `v1.1.0-rc.1` and `v1.1.0-rc.2` (checked 2026-11-24 against the contract's tags), and the contract's RFC-4 section 10 says its 1.1 tag waits for both tools to pass the runner against a release candidate. A conversation, a branch or a pull request on the contract does not meet it: the tag has to exist.

`Met-by` names the record that shows it, such as the `MILE-369` revision row that vendors the tag. `MILE-369` waits on this condition, because it cannot vendor or run a runner that is not published.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed `Open`. **Why:** the plan to conform to contract 1.1 (`MILE-369`) needs a tag that carries the runner, and none does. | **substantive** |
