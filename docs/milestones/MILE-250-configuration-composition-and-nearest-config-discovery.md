---
Stable-Id: 01M7FW0H4P1W1HTNH84E897NNQ
Status: Planned
Phase: '2'
Track: engine-extensibility
Blocked-on: null
---
# 250 — Configuration composition and nearest-config discovery

## What

Let a configuration extend another file, and find the configuration nearest the working directory so a monorepo can hold several corpora.

## Why

the audit of 2026-11-15's extensibility experiments found one configuration per git root: a nested `.urzua/config.yaml` is ignored, which contradicts the documentation and `--help`, and an organization cannot share one base configuration across repositories.

## Done means

- A decision record states the merge rule for `extends` and how protected paths apply to an extended file.
- A configuration that extends another loads with the documented merge, and a cycle or a missing file is a load error.
- A command run inside a package with its own configuration uses it, with type directories relative to that file, shown by a test.
- The documentation and `--help` describe the lookup the code performs.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
