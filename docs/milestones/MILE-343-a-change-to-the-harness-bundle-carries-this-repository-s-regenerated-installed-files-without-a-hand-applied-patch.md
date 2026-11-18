---
Stable-Id: 01M7N42HDWJD8S05QXNCX3NAE3
Status: Planned
Phase: '1'
Track: integrations
Blocked-on:
---
# 343 — A change to the harness bundle carries this repository's regenerated installed files without a hand-applied patch

## What

A pull request that changes the harness bundle can bring this repository's own installed harness files up to date without a person applying a patch by hand, and the test that compares the installed files with the bundle does not hold such a pull request red until they do.

## Why

The files `urzua harness install` writes into this repository (`.claude/`, the plugin's hooks, `.urzua/harness.lock`) are protected paths, so an agent may not edit them. A test reads them and fails when they differ from what the current bundle renders. So every change to the bundle leaves its pull request red until someone outside the agent applies the regenerated files. On 2026-11-17 one pull request needed ten such patches, each written by the agent to a file and applied by the orchestrator through the shell. While that test was red it also hid five other failing repository-wide tests for seven review rounds, because the known failure was taken to be the only one.

## Done means

- A documented way exists for the regenerated installed files to enter a pull request that changes the bundle, which does not let an agent edit protected paths by hand: either `urzua harness update` run by the agent is accepted for exactly the content the bundle renders (the gate verifies the result equals a fresh render and refuses anything else), or the comparison test renders from the bundle in a scratch directory and a separate, clearly named check reports that the committed files lag; the record of the choice says which and why.
- With that in place, a pull request that changes a hook command passes the installed-files test without any hand-applied patch, shown on a test repository and on this one.
- A protected path still cannot be changed to anything the bundle does not render, shown by a test that a hand edit is refused or reported exactly as before.
- The pull request check still names the maintainers group for a change to these paths; nothing here changes who may approve.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** one pull request on 2026-11-17 needed ten hand-applied patches to protected files, and the red test they left hid five other failures. | **structural** |
