---
Stable-Id: 01M7FVX30GN6VD7AT5B2XRJAK7
Status: Planned
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 236 — The symlink refusal covers a symlinked type directory, git's file mode and the editor hook

## What

Close the four gaps the review of the symlink refusal found: a type directory that is itself a symlink, a checkout where git records a symlink but the file system has a plain file, the editor-hook path, and the documentation of how the refusal depends on the configuration file being gated.

## Why

The independent review of `identity.record-symlink-escapes` on 2026-11-15 passed it and reproduced four cases it misses. None bypasses the gate, but `ADR-89` and `SPEC-2` claim coverage the code does not have.

## Done means

- A declared type directory, or a parent of it, that is a symlink is a blocking finding, shown by a test.
- A tracked entry with git mode 120000 is refused when `core.symlinks` is false, shown by a test.
- `check --stdin-path` on a symlinked record path runs the rule and reports the finding.
- `ADR-89` states that the default-on refusal is only as strong as the gate on the configuration file, and `urzua doctor` warns when the rule is lowered while that file is ungated.
- A test asserts that a waiver does not suppress the finding.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-15 | Initial record, `Status: Planned`. **Why:** Filed from the audits of 2026-11-15; the maintainer committed to the work in conversation that day. | **structural** |
