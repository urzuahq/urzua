---
Stable-Id: 01M7HW1SXZFP6T6S18159PZV9F
Status: Planned
Phase: '1'
Track: release-process
Blocked-on: null
---
# 294 — This repository's hooks keep working between releases

## What

Make the agent harness and git hooks in this repository use a binary that can read `main`'s configuration, without a manual build after each merge that adds a rule or key.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Twice in two days a merge added a rule the installed release did not know, and from then on every hook reported that it could not judge, until a release was cut or someone built from source by hand.

## Done means

- After a merge that adds a rule or a config key, a session in this repository judges edits without a manual build or install, shown by a documented procedure and a check in `urzua doctor`.
- `urzua doctor` reports an installed binary older than the configuration it must read and names the command that fixes it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
