---
Stable-Id: 01M7GY8BQHCMJHNR6VJ18H4FG7
Status: Planned
Phase: '2'
Track: integrations
Blocked-on: null
---
# 280 — The harness ships as an agent plugin

## What

Publish the harness bundle as a plugin for the agents that have a plugin channel, and document how an organization force-enables it so a repository setting cannot switch it off.

## Why

The harness research of 2026-11-16 compared the hook, approval, sandbox and audit features that current coding agents document. Plugins are the common distribution channel, and several agents have a managed tier that can force a plugin's hooks on; a harness committed to the repository is the easiest to install and the easiest for an agent to edit.

## Done means

- A plugin installs the same hooks as `urzua harness install`, and `urzua harness status` recognises a plugin install.
- The agent harness page documents the managed, force-enabled setup for one agent, tested.
- The plugin and the bundle are built from one source, checked in CI.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Filed from the harness research of 2026-11-16, which the maintainer asked for that day. | **structural** |
