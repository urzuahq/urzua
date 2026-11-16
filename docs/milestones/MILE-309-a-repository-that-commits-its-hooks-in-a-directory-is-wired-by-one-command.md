---
Stable-Id: 01M7K3HQ1PTCD7FNHQMQ31VYAG
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 309 — A repository that commits its hooks in a directory is wired by one command

## What

Let a configuration name the directory its git hooks are committed in, have `urzua hooks install` point the clone at it, and make every message about a missing hook name that step.

## Why

Git's `core.hooksPath` is a setting of one clone and is never cloned or fetched, so a repository that commits its hooks in a directory is unwired in every fresh clone until someone runs a repository-specific command. On 2026-11-16 this repository's main checkout had no `core.hooksPath`: the session-start report said "no pre-commit hook at .git/hooks/pre-commit -- `urzua hooks install` adds one that runs urzua", which here would write a second hook under `.git/hooks` that skips the committed hook's own guard code. With the hooks declared `managed: declared`, `urzua hooks status` said "the declaration is false -- write the hook, or remove the declaration" with `next_action: null`, although both hooks exist in the committed directory and the fix is one git setting. The same report ended with a paragraph about `on_unjudged: deny` refusing an agent's edits, which is not a consequence of a missing git hook.

## Done means

- A configuration can name the directory its hooks are committed in, and `urzua hooks install` sets this clone's `core.hooksPath` to it, refusing with the current value when another path is already set.
- When the named directory holds the hooks and the clone is not pointed at it, `urzua hooks status`, `urzua doctor` and the session-start report say so in those words and carry a `next_action` that is the one command to run.
- No message recommends writing a hook under `.git/hooks` in a repository whose configuration names a hook directory or declares the hook.
- The session-start report states a consequence only for the problems it lists: the `on_unjudged` paragraph appears only when a listed problem leaves a guardrail unable to judge.
- This repository's configuration names `.git-hooks`, and its contributor guide gives `urzua hooks install` as the setup step.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** The maintainer reported on 2026-11-16, with a screenshot of the session-start warning in this repository, "we found another apparent potential usability bug that we need to fix"; and said earlier that day that friction met using the tool is an improvement point. | **structural** |
