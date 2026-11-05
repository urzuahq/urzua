---
Stable-Id: 01M6PZ740V8PWKS22DZ27QKXV5
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-89
Depends-on: MILE-198
Blocked-on: null
---
# 204 — Run this repository on the installed harness bundle and delete scripts/hooks

## What

Switch this repository from the hand-wired editor hook to the bundle `urzua harness install` writes: declare `harness` with `claude-code` in `.urzua/config.yaml`, install and commit the bundle and its lock, and delete `scripts/hooks/record-edit-hook.sh` and `scripts/hooks/claude-code-record-edit.py` together with the tests that pin them, moved to the engine cores of `MILE-198`.

## Why

`ADR-89` makes the editor hook an engine subcommand, so the shell and Python chain (with the fail-open paths in its Context) has no reason to stay. Split from `MILE-198` after review, because building the mechanism and cutting this repository over are separate changes and the second is the proof that the first works on a real checkout. It is blocked until `MILE-198` lands.

## Done means

- `.urzua/config.yaml` declares `harness` with `claude-code`; `urzua harness status` reports no drift and exits 0; `.claude/settings.json` holds only the merged urzua hook entries and the entries it held before.
- `scripts/hooks/*` is deleted and nothing in the repository, the Makefile, the git hooks or the docs refers to it; the tests that pinned the scripts' verdicts run against the engine cores and keep their planted cases.
- `urzua check` and `urzua check --base origin/main` report 0 unwaived errors or warnings, and a planted bad record edit through the Edit tool is refused or reported by the installed hook in a session.
- `SPEC-19` is revised to describe this repository as running the installed bundle, with a revision row.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed, split from `MILE-198`. **Why:** Independent review of pull request 355 found `MILE-198` too large; the cutover of this repository and the deletion of `scripts/hooks` are its own change. Blocked on the maintainer accepting `ADR-89`. | **substantive** |
