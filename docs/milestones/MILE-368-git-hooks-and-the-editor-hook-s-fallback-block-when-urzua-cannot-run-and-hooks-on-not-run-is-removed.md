---
Stable-Id: 01M85PWBY37W74A3Q0Q97CHTZS
Status: Planned
Phase: '1'
Track: integrations
Implements: ADR-107
Depends-on: MILE-359
Blocked-on: ADR-107 being accepted by the maintainer (it is Proposed)
---
# 368 — Git hooks and the editor hook's fallback block when urzua cannot run, and hooks.on_not_run is removed

## What

Implements `ADR-107`: this repository's `pre-commit` and `pre-push` hooks and the hooks `urzua hooks install` writes exit 2 with the contract's `not-run` literal when urzua cannot run; the editor hook's fallback refuses in the case it lets through today; and `hooks.on_not_run`, `URZUA_HOOKS_ON_NOT_RUN` and the `hooks run` exit code 3 are removed.

## Why

Contract 1.1 (the published agent CLI contract's RFC-4) is taken whole: urzua prints `contract_version` `"1.1"` only when every piece below is delivered (`MILE-370`). `MILE-179` decided the opposite and its Done text stays; `ADR-107` supersedes that choice once the maintainer accepts it. The edits to `.git-hooks/pre-commit`, `.git-hooks/pre-push` and the regenerated `.claude/settings.json` are the maintainer's (protected paths); the code and tests are not. This milestone waits for the ADR and for the `not-run` literal's keys, which `MILE-365` and `MILE-367` define.

## Done means

- The hook text `urzua hooks install` writes (`shell_line`, `shell_block`, `framework_entry`) prints the `not-run` literal and exits 2 when urzua is not found or cannot start, and a test with the binary taken away shows exit 2 and that stdout and stderr hold only that one JSON object, for the shell, framework and `hooks run` forms.
- `pre_commit_hook_says_loudly_when_the_binary_is_absent` is renamed or rewritten to assert exit 2 and the literal, and a test shows a blocking finding still exits 1 and a clean run exits 0; the first assertion fails against `.git-hooks/pre-commit` as it is today.
- A grep of `rust/crates/*/src` and `docs/` finds no `on_not_run`, `URZUA_HOOKS_ON_NOT_RUN` or `NOT_RUN_BLOCK`, and a configuration that still names `hooks.on_not_run` is handled as `SPEC-3` states for an unknown key, shown by a test.
- The editor-hook fallback refuses a call in a governed session even when the configuration reads with certainty as open, shown by a test that fails against the current allow-with-a-notice; `on_unjudged: allow` still lets through a case where the binary runs and has no opinion, shown by a test.
- `.git-hooks/pre-commit`, `.git-hooks/pre-push` and `.claude/settings.json` carry the edits listed in `ADR-107` (the maintainer's), and `urzua doctor` reports both hooks wired in this repository.
- `ADR-89` and `ADR-71` carry the dated amendments the maintainer accepts, and `SPEC-3`, `SPEC-15`, `SPEC-19`, `docs/reference/ci.md`, `use-cases.md`, `adopt.md`, `commands.md` and `agent-harness.md` no longer describe warn-and-exit-0; a grep of `docs/` for `lets the commit through` and `NOT checked` finds none left in force.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted (relayed 2026-11-24: "accepted [the contract's] RFC-4 at 2026-11-24 00:02 UTC"). | **substantive** |
