---
Stable-Id: 01M6NX35EBXRAW2CHPRYHBZ6AF
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-89
Blocked-on: null
---
# 198 — Build urzua harness install, the engine-run hook cores and the Claude Code adapter

## What

Build the install mechanism and the engine-run cores of `ADR-89`: `urzua harness install [--adapter NAME] [--dry-run]`, `status`, `update` and `adapters`; the `harness` configuration key with a per-guardrail `enabled` and `on_unjudged`; the lock file `.urzua/harness.lock`; the cores `urzua harness run edit` and `urzua harness run session-start` speaking the neutral request and verdict; and the Claude Code adapter (settings fragment and one translator). The bundle is embedded in the binary and versioned with the engine. It is blocked until the maintainer accepts `ADR-89`.

## Why

The editor hook today is a `jq`, `bash` and `python3` chain with several paths that fail open and tell the agent nothing (`scripts/hooks/record-edit-hook.sh:69-71`, `scripts/hooks/claude-code-record-edit.py:55`), it exists only in this checkout, and no other repository can install it (`ADR-89`, Context). Phase 0 is urzua reproducing, in its own repository, the governance functionality a real adopter environment has (`SPEC-1`, Exit bar); an agent harness that is installed as one bundle and checked at session start is part of that functionality, and the maintainer decided on 2026-11-05 that it belongs in Phase 0.

## Done means

- `urzua harness install --adapter claude-code` in an empty repository with a `.urzua/config.yaml` writes the adapter's settings fragment, translator and `.urzua/harness.lock`, and `--dry-run` writes nothing and lists the same files. `status` reports a missing, a modified and an out-of-date file, each planted in a test that fails before the change; `update` rewrites an unmodified file and reports, without overwriting, a modified one unless `--force`.
- `harness.guardrails` loads from the configuration; an unknown guardrail or key, and a guardrail the adapter cannot provide, is reported at install and by `urzua doctor`, never skipped. An absent `harness` key installs and enables nothing (a test pins it).
- `urzua harness run edit` reproduces every verdict of `scripts/hooks/record-edit-hook.sh` (`allow`, `allow-with-warnings`, `block`, `allow-unjudged`, `allow-override`, `not-governed`) on the existing planted cases, and a missing binary, a missing `jq` equivalent and an unreadable payload each give the stated verdict with a message to the agent, not a silent exit.
- `urzua harness run session-start` reports a missing or version-mismatched binary, an unwired hook entry, an unwired git hook and a bundle older than the binary, each planted; no engine source file contains an agent vendor's name, and a test scans for it.
- This repository runs the installed bundle: `.urzua/config.yaml` declares `harness` with `claude-code`, the old `scripts/hooks/*` cores are deleted, `urzua harness status` reports no drift, and `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings.
- `SPEC-3` documents `harness` and `SPEC-19` the bundle, each with a revision row; a new spec records the neutral request, verdict and adapter layout; `make rule-table-check` passes; a minor changeset names the new commands.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
