---
Stable-Id: 01M6K1Y9EKS4A75KJWY8ZK18EM
Status: Planned
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
---
# 169 — field.pending defers a pending value until the status that requires the field

## What

Give `field.pending` an opt-in `pending_until_required` option: a pending value in a field the type's `statuses.require` ties to certain statuses is not reported while the record's status is not one of them. A field no `require` entry names is judged as before.

## Why

A placeholder `Regression-test` ("not yet written", "planned: ...") is a legitimate state while a bug is `Open` and a real gap once it is `Fixed`. `status.required-field-missing` already flags only the `Fixed` ones, but `field.pending` read the same `statuses.pending` list with no status scoping and warned on about twenty `Open` bugs. Pending is a legitimate state until the record reaches the status that requires the field; the type already declares which status that is.

## Done means

With the option declared `true`, a pending value in a required-at-status field is not reported before that status and is reported at it; a skipped slot stays eligible and is counted unexamined, so it never disappears from the population; a field no `require` entry names keeps its behaviour; the option is refused on any other rule at load; and this repository's `bug` type declares the placeholder patterns and `require: [{when: [Fixed], fields: [Regression-test]}]` so `urzua check` exercises it.

The option is off by default. Making it the default would silently stop warning on every existing repository whose `require` already names a field it also holds as pending at an earlier status (a `Draft` ADR with `Deciders: TBD`), which is a behaviour change nobody asked for; the declaration is the type's, the opt-in is the rule's, as with `allow_extension_prefix` (`MILE-141`).

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and built: `field.pending` takes `pending_until_required` (`SPEC-2` 0.57, `SPEC-3` 0.45); refused on any other rule at load. Exercised by this repository's `bug` type. **Why:** the Done means above; `MILE-162` built the pattern entries this relies on. | **substantive** |
