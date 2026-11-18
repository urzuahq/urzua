---
Stable-Id: 01M7QDN4MQ4RZZRGRVQREN71M0
Status: Planned
Phase: '1'
Track: release-process
Blocked-on:
---
# 345 — CI checks that the latest released binary can load this repository's current config

## What

Continuous integration runs the latest released urzua binary against this repository's current `.urzua/config.yaml` and fails when that binary cannot load it.

## Why

On 2026-11-18 a pull request added two config keys that only the code on its own branch understood (`export` and `part_of`) to this repository's config. All 17 checks passed. The released 0.4.10 binary refused the config with `unknown field export`, so every command and every hook in this repository would have failed for anyone on the release until the next one shipped. It was caught by hand before the merge and the config change was reverted. The same thing happened earlier with `hooks.path`. The `gate (pinned release)` job does not cover this: it runs the pinned release against the records as they were at that release's tag, not against the config on the pull request.

## Done means

- A CI job installs the latest published release the way an adopter does and runs a config-loading command (for example `urzua check`) against the pull request's `.urzua/config.yaml`; the job fails when the binary reports a config error.
- A test or a recorded CI run shows the job failing on a config that carries a top-level key the latest release does not know, and passing on the config at the current main.
- The job's failure message names the release version it ran and the key it refused.
- `CONTRIBUTING.md` or the release documentation says a new config key enters this repository's own config only after a release that understands it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-18 | Initial record, `Status: Planned`. **Why:** a config change that the released 0.4.10 could not load passed every check on 2026-11-18 and was caught by hand. | **structural** |
