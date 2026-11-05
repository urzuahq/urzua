---
Stable-Id: 01M6MP8RMWGBF9ZH42721R8CKS
Status: Planned
Phase: '0'
Track: governance-process
Implements: ADR-78
Blocked-on: —
---
# 189 — An opt-in warning flags a Realized-by locator on a file under back_pointer_paths and nudges toward an Implements or Verifies comment

## What

An opt-in warning rule that flags a `Realized-by` locator naming a file under `back_pointer_paths`, with an `edit-field` suggested action (`ADR-73`) that moves the evidence to an `Implements:` or `Verifies:` comment in that file. It is a nudge and not an error: `Realized-by` stays the only option for a path the comment scan cannot reach.

## Why

`ADR-78` item 4 satisfies `RFC-7` section 5, preferring the lighter mechanism: a comment in the file is scanned automatically and cannot go stale away from the code, where a `Realized-by` locator is hand-maintained. No check exists, and `ADR-78` notes that no milestone yet builds it.

## Done means

- A rule in the rule table, off unless declared, warns on a `Realized-by` locator whose path is under a declared `back_pointer_paths` entry, naming the record, the locator and the path, with an `edit-field` suggested action, and reports nothing for a locator on a path outside every `back_pointer_paths` entry.
- A planted-violation test fails before the rule exists and passes after; a locator the comment scan cannot reach is not flagged.
- The rule is enabled in this repository's `.urzua/config.yaml`, and what it reports on this corpus is recorded in the pull request and acted on or left as findings by decision, not hidden by widening a list.
- `SPEC-2` and the generated rule table describe the rule, and `make rule-table-check` passes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-78` Accepted 2026-11-05 decides `RFC-7` section 5 as an opt-in warning rule and notes that no milestone exists for it. | **substantive** |
