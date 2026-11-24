---
Stable-Id: 01M85PWBY37W74A3Q0Q97CHTZS
Status: Planned
Phase: '1'
Track: integrations
Implements: ADR-107
Depends-on: MILE-359, MILE-365, MILE-367
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
- A test takes the binary away and a second runs a `check` that exits 2 (urzua died without a report, or answered `not-run`), through the installed shell hook and `hooks run`: both exit 2 with the `not-run` literal and nothing else, where the installed `shell_line` today ends `|| exit 1` and `.git-hooks/pre-commit`'s `*)` branch exits 0.
- A pre-push test (there is none for the missing-binary case today) shows `.git-hooks/pre-push` and the pre-push hook `urzua hooks install` writes exit 2 with the literal when the binary is absent or `check` exits 2, and exit 1 on a blocking finding.
- A blocking git-hook run prints urzua's JSON report and no banner, count line, `suggested:` line or closing line, shown by a test; `blocked()` and the `no blocking findings` line in `hooks.rs` (lines 2299 and 2343 to 2356 today) print only the report, and the test fails against them.
- The editor hook's missing-binary refusal (the text in `.claude/settings.json`'s command and the bundle that generates it) is the `not-run` literal in the host's refusing form, and a test with the binary absent shows no prose around it.
- The post-command check (`harness/post_command.rs`, `SPEC-19` lines 127 to 139) states a `git` that is absent, a configuration that does not load or a `check` that cannot run as the `not-run` report in the host's format and does not block, since a hook that runs after the fact cannot (RFC-4 section 5.1), shown by a test; `ADR-107` records the question.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Filed as planned work: nothing built, no evidence yet. It is one piece of conforming to the published agent CLI contract 1.1, whose RFC-4 the maintainer accepted: "I accept RFC, and yes allow option can be outside rule" (the contract's RFC-4, revision row of 2026-11-24). | **substantive** |
