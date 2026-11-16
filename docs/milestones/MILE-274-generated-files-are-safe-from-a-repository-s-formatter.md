---
Stable-Id: 01M7GX8BQ5QVV4ZQDK3KGM60KK
Status: Planned
Phase: '1'
Track: integrations
Blocked-on: null
---
# 274 — Generated files are safe from a repository's formatter

## What

Stop a code formatter from rewriting the files urzua generates and compares byte for byte (the known-references file, the harness lock, generated indexes): `init` and `harness install` add them to the formatter's ignore file when one exists, `doctor` reports a formatter that would touch them, and the documentation says what to ignore.

## Why

An agent working in an adopting repository reported on 2026-11-16 that `.urzua/known-refs.json` was missing from the formatter's ignore file: the formatter reflowed it and the next run saw a changed file. Nothing in `init`, `doctor` or the documentation mentions formatters.

## Done means

- `urzua doctor` reports each generated file that a formatter configured in the repository would rewrite, naming the file and the line to add to the ignore file.
- `urzua init` and `urzua harness install` add those lines when the repository has a formatter ignore file, and say so in their report.
- `refs refresh` writes a file a common JSON formatter leaves unchanged, or the file is documented as ignored, shown by a test that formats it and compares.
- The adopt page lists the generated files and what to ignore for the common formatters.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Reported from an adopting repository on 2026-11-16; the maintainer asked for setup or directions. | **structural** |
