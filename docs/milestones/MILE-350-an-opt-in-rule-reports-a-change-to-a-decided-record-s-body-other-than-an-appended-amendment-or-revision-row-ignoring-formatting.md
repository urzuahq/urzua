---
Stable-Id: 01M7SKPS79R5P9DWR4EWD95XH1
Status: Planned
Phase: '1'
Track: schema-governance
Blocked-on:
---
# 350 — An opt-in rule reports a change to a decided record's body other than an appended amendment or revision row, ignoring formatting

## What

An opt-in `check` rule, run under `--base`, that reports a change to the body of a record in a decided status when the change is anything other than an appended dated `## Amendment` section or an appended revision-log row. It compares the body against the merge base's copy with whitespace and table-cell padding normalised, so a formatter's re-alignment alone never trips it. Header fields are left to the rules that already judge them.

## Why

A formatter run over a repository re-aligned a whole table in an Accepted ADR when one header line was added, and nothing reported that the body of a decided record had changed. The rule that holds a decided record's body still today is the instruction not to edit it, and no check enforces it. The maintainer chose this direction on 2026-11-19 by picking "Opt-in content rule" from the options offered.

Until it exists, add the record directories to the formatter's ignore file, so a formatter never rewrites a decided record.

No decision record exists for the rule yet, so `Implements` is not set.

## Done means

- With the rule enabled and `--base` given, a record in a decided status whose body differs from the base's copy by anything other than an appended dated amendment section or appended revision-log rows is a finding on the record naming the first changed line; shown by tests through the built binary that fail with the rule off.
- A change that only re-aligns a table, re-wraps whitespace or re-pads table cells reports nothing; shown by a test that runs a formatter-style rewrite of an Accepted record.
- An appended `## Amendment (YYYY-MM-DD)` section and an appended revision-log row each report nothing; a changed or removed existing revision-log row, or an amendment edited after it was added, is reported.
- A record in a status that is not decided, and a header-only change, report nothing from this rule; header fields stay with the rules that judge them.
- The rule is off unless enabled, `check` without `--base` says it did not run and why, and a repository that does not enable it sees no finding and no notice.
- The capability is exercised in urzua's own `.urzua/config.yaml` and corpus, and `make rule-table-check` passes with the rule in SPEC-2's generated table.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-20 | Partly delivered by `section.decided-body-changed` (`decision_types`, `appendable_sections`, `exempt_sections`; `SPEC-3`, "Decided records keep their body"): a changed decided body, an edited amendment and a changed or removed revision row are reported on a `--base` run, and a reflow, a re-padded table and a record not decided at the merge base are not, shown by tests through the built binary. A section added whole after the decision counts as appended, since an amendment sits after the decision it amends. **Remains:** the rule is not declared in `.urzua/config.yaml`, a protected path the maintainer edits, so "exercised in urzua's own configuration and corpus" is open; no `Realized-by` is set and `Status` stays `Planned`. **Why:** the build of the direction chosen on 2026-11-19 ("Opt-in content rule"). | **structural** |
