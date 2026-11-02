---
Stable-Id: 01M6D2KHK1DJNCNTC7R5VSSZAS
Status: Open
Found-in: "A /code-review v0.3.0...main pass, round 29"
Regression-test: "planned: a check test with two types sharing a dir, asserting the overlap is rejected or disclosed"
---
# 168 — two record types may declare the same dir, so every record is loaded and validated once per type

## What was wrong

Config validation does not reject, warn about, or disclose two record types declaring the same `dir`.
`load_records` loops over types and discovered files, so each file under the shared directory becomes a
record once per type and is validated against both.

Repro: config with `adr` and `rfc` both `dir: docs/adr`, `rfc` requiring `[Status, Foo]`, three ADR files.
`urzua check` reports `files_examined: 6` for 3 files and `missing required header field 'Foo' for record
type 'rfc'` three times on files that are ADRs. `urzua doctor` reports both `record-type-dir ... exists`
as ok and never mentions the overlap.

Adjacent: `dir: ./docs/adr` (leading `./`) parses, matches nothing (path equality is component-wise and
keeps the `.`), and the run is `not-run` with no reason unless the opt-in `type.dir-matches-nothing` is
on. `dir: docs/adr/` works.

## What the docs say

`RFC-35` states "a directory is a type", but nothing enforces it. `BUG-54` is the inverse case (a nested
dir). No ADR decides what two types over one directory should mean.

## Decision needed

- Reject at config load (a config error, exit 2), or warn through a notice or `doctor` finding and keep
  loading? Rejecting is consistent with "a directory is a type"; warning keeps a deliberate overlap
  possible, which nobody has asked for.
- Whether `files_examined` should count files or (file, type) pairs once an overlap is allowed at all.
- Whether un-normalised dirs (`./docs/adr`) should be normalised at parse time (mechanical) or rejected.

## References

- `RFC-35` -- "a directory is a type".
- `BUG-54` -- nested-dir inverse case.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** found by a full-release code review and reproduced with the release binary on a disposable fixture. Left `Status: Open` -- the fix needs a decision, not a quick edit. | **substantive** |
