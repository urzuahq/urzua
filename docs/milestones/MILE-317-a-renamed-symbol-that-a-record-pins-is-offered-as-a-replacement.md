---
Stable-Id: 01M7K8MD99KQKG4N6ATDP2B3YC
Status: Planned
Phase: '1'
Track: write-path
Blocked-on: null
---
# 317 — A renamed symbol that a record pins is offered as a replacement

## What

When a scoped pin's symbol no longer exists in its file, have `urzua fix` say so with a runnable next action that replaces it, listing the symbols the file now has that are close.

## Why

On 2026-11-16 a builder renamed a test function that a record pins by name. `urzua fix --rebaseline` answered that the name "matches nothing" with no next action, and the builder kept a test name that no longer describes the test rather than work out the replace command. The tool has the replace operation (`fix --pin --replace`); the message did not lead to it.

## Done means

- A scoped pin whose symbol is gone is reported with a `next_action` that is the `fix --pin --replace` command with the old locator filled in, shown by a test.
- When the pinned file has exactly one new symbol whose body hashes to the pinned hash, the replacement is offered with that symbol filled in.
- The finding is the same under `check`, so the hooks lead to the same command.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Met on 2026-11-16 by a builder using the tool on this repository's own records; the maintainer said that day that friction met using the tool is an improvement point. | **structural** |
