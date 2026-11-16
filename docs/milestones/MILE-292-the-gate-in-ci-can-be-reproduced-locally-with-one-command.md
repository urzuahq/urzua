---
Stable-Id: 01M7HW17C5K8H2MTRQ3DKMQKW8
Status: Planned
Phase: '1'
Track: discoverability
Blocked-on: null
---
# 292 — The gate in CI can be reproduced locally with one command

## What

Give a contributor one command that runs exactly what the pull request gate runs, including the actors file, before pushing.

## Why

Found on 2026-11-16 while urzua's own builders and reviewers used the tool to land a week of pull requests. Builders' local `check --base` passed and the pull request then failed, because the gate runs with an actors file built from the pull request, which needs its number. Each such miss cost a push and a CI cycle.

## Done means

- One documented command runs the gate's checks for the current branch against a base, building the actors input the gate would build, and exits as the gate would.
- Before a pull request exists, the command says which checks it could not run and why.
- The CI reference page names the command.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from friction met while using urzua on its own repository on 2026-11-16; the maintainer said that day that issues met in using and adopting the tool are improvement points. | **structural** |
