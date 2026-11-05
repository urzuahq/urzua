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

Build the install mechanism and the engine-run cores of `ADR-89`: `urzua harness install [--adapter NAME] [--dry-run]`, `status`, `update`, `uninstall` and `adapters`, with the settings-merge rules of its section 2; the `harness` configuration key with a per-guardrail `enabled` and `on_unjudged`; the lock file `.urzua/harness.lock`; the cores `urzua harness run edit` and `urzua harness run session-start` speaking the neutral request and verdict; and the Claude Code adapter (settings fragment and one translator). The bundle is embedded in the binary and versioned with the engine. Moving this repository onto the installed bundle and deleting `scripts/hooks` is `MILE-204`, split off so this milestone is the mechanism and that one is the cutover. It is blocked until the maintainer accepts `ADR-89`.

## Why

The editor hook today is a `jq`, `bash` and `python3` chain with several paths that fail open and tell the agent nothing (`scripts/hooks/record-edit-hook.sh:71` for `jq`, `:86` for the binary through `unjudged()` at `:69`; `scripts/hooks/claude-code-record-edit.py:55` for the tool filter). An unset `CLAUDE_PROJECT_DIR` makes `python3` exit 2, which blocks every edit with no stated reason; that is fixed here too, it exists only in this checkout, and no other repository can install it (`ADR-89`, Context). Phase 0 is urzua reproducing, in its own repository, the governance functionality a real adopter environment has (`SPEC-1`, Exit bar); an agent harness that is installed as one bundle and checked at session start is part of that functionality, and the maintainer decided on 2026-11-05 that it belongs in Phase 0.

## Done means

- `urzua harness install --adapter claude-code` in an empty repository with a `.urzua/config.yaml` writes the adapter's settings fragment, translator and `.urzua/harness.lock`, and `--dry-run` writes nothing and lists the same files. `status` reports a missing, a modified and an out-of-date file, each planted in a test that fails before the change; `update` rewrites an unmodified file and reports, without overwriting, a modified one unless `--force`; `status` exits 1 on drift; `uninstall` removes the files that match the lock, removes only the urzua hook entries and the marked instructions block from shared files, and reports a modified file without deleting it.
- `harness.guardrails` loads from the configuration; an unknown guardrail or key, and a guardrail the adapter cannot provide, is reported at install and by `urzua doctor`, never skipped. An absent `harness` key installs and enables nothing (a test pins it). `acceptance` or `protected-paths` enabled with no `authority` key is a load error naming the guardrail.
- `urzua harness run edit` reproduces every verdict of `scripts/hooks/record-edit-hook.sh` (`allow`, `allow-with-warnings`, `block`, `allow-unjudged`, `allow-override`, `not-governed`) on the existing planted cases, and a missing binary, a missing `jq` equivalent and an unreadable payload each give the stated verdict with a message to the agent, not a silent exit.
- Installing into a repository that already has a `.claude/settings.json` merges the hook entries and leaves every other key and entry byte-identical; a second install changes nothing; an invalid JSON file, or an edited entry with the same key, is refused with `not-run`. The agent-instructions block is rewritten only between its markers and is identical after two installs. Each is a planted test.
- `urzua harness run session-start` reports a missing or version-mismatched binary, a missing interpreter, an unset project-directory variable, an unwired hook entry, an unwired git hook and a bundle older than the binary, each planted; with the variable unset the adapter's command still yields a stated verdict and not a bare exit 2. No engine source file contains an agent vendor's name, and a test scans for it with the embedded bundle's adapter directory excluded (the adapter names its vendor by design).
- `on_unjudged: deny` on an unjudged guardrail refuses an `agent` request and allows a `human` request, and the message and `urzua doctor` name the escape hatch (a person edits outside the harness); a test pins all three.
- `SPEC-3` documents `harness` (including `instructions_file`) and `SPEC-19` the bundle, each with a revision row; a new spec records the neutral request, verdict and adapter layout; `make rule-table-check` passes; a minor changeset names the new commands.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-89` (Proposed) decides the installable agent harness bundle on maintainer decisions of 2026-11-05 and no milestone builds this part; blocked on the maintainer accepting it. | **substantive** |
> | 2026-11-05 | Split and corrected after review. The cutover of this repository (delete `scripts/hooks`, run the installed bundle) moves to `MILE-204`; `uninstall`, the settings-merge rules, the `CLAUDE_PROJECT_DIR` fix, the `on_unjudged: deny` escape hatch, the `authority` load error and the vendor-scan exclusion for the bundle are added; code citations corrected (`record-edit-hook.sh:71`, `:86`). **Why:** Independent review of pull request 355; the Done means were too large for one change and several were untestable as written. Still `Planned`, blocked on `ADR-89`. | **substantive** |
