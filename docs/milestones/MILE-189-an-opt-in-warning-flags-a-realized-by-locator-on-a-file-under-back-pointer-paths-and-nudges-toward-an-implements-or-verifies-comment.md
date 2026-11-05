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
> | 2026-11-05 | The rule is built as `embodiment.locator-on-back-pointer-path` (`SPEC-2`, `SPEC-3`) in #310: a `code:` locator on a file under `back_pointer_paths` and a `test:` locator on a file under `test_paths` are warned, with an `edit-field` action; a `spec:` locator, a `test:` locator outside `test_paths` and a scoped or pinned locator are left. Not enabled in this repository's `.urzua/config.yaml` yet, so `Status` stays `Planned` and no `Realized-by` is set: enabled over this corpus it reports 308 warnings on 129 records (188 `code:` and 120 `test:` locators, 265 of them on milestones), and this repository fails CI on any warning (`fail_on: warning`). Whether to move that evidence into `Implements:` and `Verifies:` comments, waive it for a period, or keep the rule off here is the maintainer's decision; enabling it is one entry under `rules:` with `back_pointer_paths` and `test_paths` as `embodiment.back-pointer-resolution` declares them. **Why:** Done means asks for the rule enabled in this repository's own configuration, and that cannot be done without either red CI or a decision this milestone does not carry. | **substantive** |
