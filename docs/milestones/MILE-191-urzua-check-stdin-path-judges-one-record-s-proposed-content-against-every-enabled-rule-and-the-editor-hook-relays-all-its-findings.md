---
Stable-Id: 01M6MP8P2DDJ4VGWQ3QAMAES1Y
Status: Done
Phase: '0'
Track: governance-process
Implements: ADR-78
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/check.rs, code:rust/crates/urzua-core/src/rules/runner.rs, code:scripts/hooks/record-edit-hook.sh, code:scripts/hooks/claude-code-record-edit.py, test:rust/crates/urzua-cli/tests/check_stdin_integration.rs, test:rust/crates/urzua-cli/tests/record_edit_hook_integration.rs
---
# 191 — urzua check --stdin-path judges one record's proposed content against every enabled rule, and the editor hook relays all its findings

## What

`urzua check --stdin-path <path>`: read the proposed content of one record from stdin and judge it against every enabled rule, without writing anything, and report the findings on that record. The editor hook (`scripts/hooks/record-edit-hook.sh` and the Claude Code adapter) relays that report instead of `urzua fix --stdin-path`'s: an error blocks the edit, a warning is shown and does not, and the loud "not judged" message stays for every way `check` cannot run. The hook keeps the named override and its sub-second budget. `urzua-core` stays pure: the content arrives as data, and every read lives in `urzua-cli`.

## Why

`ADR-78` item 2 decides that the pre-write hook judges every rule on the edited record, not only the computed `Embodiment` that `urzua fix --stdin-path` reports (`ADR-69`, `MILE-178`). Today a header, field, section or status error in the same edit passes the editor and is caught only at the commit and in CI (`ADR-71`). `fix --stdin-path` (`MILE-159`) is the prospective-content form for `fix`; `check` has none. `ADR-78` notes that no milestone yet builds this.

## Done means

- `urzua check --stdin-path <path>` reads proposed content from stdin and reports `check`'s findings for that record against every enabled rule that can judge one record (a rule that compares with a committed copy, `status.transition-allowed` and `revision-log.version-bump-required`, is judged against HEAD's copy), writes nothing, and exits as `check` does; the path need not exist yet; a path no record type governs is a could-not-run report naming it; combining it with `PATHS` or `--base` is refused.
- A planted record edit with a bad header, a missing required section and a status error is reported by `check --stdin-path` with the rule that found each, and the same content that is valid reports none.
- The hook blocks on an unwaived error finding on the edited record, shows warnings without blocking, allows an edit whose only errors were already on the file on disk (named as pre-existing), and says plainly that it did not judge when `urzua` is missing, returns could-not-run or cannot read history. The named override is kept and appears in the hook's output.
- The hook's allow message counts the rules that ran and names each enabled rule it could not judge, never claiming every rule. A single-record judgement through the core hook completes under a second on this repository's corpus, including an edit with errors, measured and recorded in `SPEC-8`'s or `SPEC-2`'s account of the hook, and the hook is tested with recorded Write, Edit and MultiEdit payloads.
- `.claude/settings.json` and this repository's records are what the hook runs against, and `SPEC-2` and `SPEC-8` describe `--stdin-path` on `check`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-78` Accepted 2026-11-05 decides the hook judges every rule on the edited record and notes that no milestone exists for it; it needs the prospective-content form of `check`. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-05 | Built in #308: `urzua check --stdin-path` (`SPEC-2`), `Realized-by` added, and the hook relays it (`SPEC-8`); measured about 0.6s per record through the core and about 1.1s when the edit has errors. `Status` set to `Done`. **Why:** every item of Done means is met and exercised in this repository's configuration and corpus. | **substantive** |
> | 2026-11-05 | Fixed after review of #308: the hook's `allow` message counts the rules that ran and names the enabled rules a single record cannot be judged by, instead of claiming every enabled rule; `status.transition-allowed` and `revision-log.version-bump-required` are judged against HEAD's copy of the edited record; pending patterns are compiled once per process, which brought a blocked edit from about 1.1s to about 0.77s (best of 20 runs, loaded 4-core machine; clean edit about 0.43s). **Why:** the reviewer found the message untrue (20 enabled rules were not run) and the blocked path over a second. | **substantive** |
> | 2026-11-05 | Renumbered from `MILE-187` to `MILE-191`: another milestone took `MILE-187` on `main` first, and every reference here, in the specs and in the code now says `MILE-191`. **Why:** two records claimed one identifier (`identity.collision`). | **structural** |
