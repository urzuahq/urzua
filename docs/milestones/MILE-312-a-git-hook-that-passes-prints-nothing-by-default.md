---
Stable-Id: 01M7K4YQWMNPPTA3YGT99NPB5C
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 312 — A git hook that passes prints nothing by default

## What

Make `urzua hooks run` silent when nothing blocks, with a configuration key that turns its success line back on.

## Why

`urzua hooks run` prints "no blocking findings" on every clean commit and push. The agent harness hooks are quiet by default since the maintainer asked for that on 2026-11-16; the git hooks are read by the same agents, who pay for every line.

## Done means

- `urzua hooks run` prints nothing and exits zero when no finding blocks, shown by a test.
- A configuration key restores the success line, and the configuration reference names it.
- A refusal is unchanged: the count, one line per blocking finding, the suggested next action and the command for the full report.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found on 2026-11-16 by an independent review of a records pull request, reading `urzua hooks run`; the maintainer said that day "yes hook should be silent by default and in config" about the agent harness hooks. | **structural** |
