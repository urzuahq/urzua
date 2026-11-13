---
Stable-Id: 01M2W0Q9N0GT6WTZPX9YDJKS5W
Status: Fixed
Found-in: 'A code review of PR #74 -- narrowing SPEC-1 deleted its revision log along with a section, and no rule reported it'
Realized-by: test:rust/crates/urzua-core/src/rules/revision_log.rs#a_record_of_a_type_declaring_required_false_with_no_log_is_absent_and_silent@sha256:ff5c3d2d3c9bdcd22a9d6d0a4375c9bd2f18ef948b59621d0cd56f4a34ebb343, test:rust/crates/urzua-cli/tests/check_integration.rs#a_record_with_no_revision_log_stays_unexamined_when_its_type_declares_required_false@sha256:8c21b9a20b5395a0d2d3321510ca7aa4d8dcec215e9bd054bdef7ca861a045e9, test:rust/crates/urzua-core/src/rules/revision_log.rs#a_record_of_a_type_declaring_required_true_with_no_revision_log_section_is_a_finding@sha256:75ce5b4681d0aa830ae994977f0951816d02d02512fc1add9d93b78c57e73836, test:rust/crates/urzua-cli/tests/check_integration.rs#a_record_with_no_revision_log_is_a_finding_when_its_type_declares_one_required@sha256:6437488c307ab230a19a18f14610ecc5697e0e1eee1df9675651565287e67162
Blocked-on: —
---
# 50 — A record with no revision log silently leaves `revision-log.change-class-required`'s scope

## Why `MILE-4`, not `MILE-98`

Repointed 2026-09-20. This record asks two things and only one is the document model's.

*Where* a revision log lives, and how a section is delimited, is `MILE-98`'s `sections.from`.
*Whether a type must have one* is policy -- a per-type declaration parallel to `required_fields` --
and that is `MILE-4`'s `required_sections`, which does not exist in the config schema today.

This record's own `Regression-test` field already stated the split before either half was placed:
*"a record of a type that carries revision logs, with the block absent, must be a finding; and a
record of a type that does not carry them must not be."* That is `required_sections`, described
without being named.

`MILE-4` is itself blocked on `MILE-98`, so the chain is unchanged; the pointer simply skipped a link.

## What is wrong

`find_revision_log_entries` keys on the literal string `> **Revision log**`. A record without that
marker returns `None`, and the rule moves on. **Absence is indistinguishable from compliance.**

A record can therefore exit the rule's scope by losing one line, which is exactly what happened.
Narrowing `SPEC-1` deleted its "Monorepo layout" section and took the revision-log block with it:

```text
SPEC-1 revision rows on main:  22
SPEC-1 revision rows after:     4   (marker gone; the four dangled after ## References)
```

`revision-log.change-class-required` is declared `error` in this repository. It stayed green. The loss
was found by review, not by the tool that exists to find it.

## The scope this hides

Measured 2026-09-19, and the three numbers do not agree, which is itself the point: **264 record files
exist, 68 carry no revision-log block, and the rule reports `records_examined: 196`**. 264 - 68 is 196,
so the arithmetic holds -- what is missing is any statement of it. The report shows 196 and says
nothing about the 68 -- which is `BUG-40`'s
reporting gap made concrete.

Some of those 68 are legitimate: a `waiver` may have nothing to log. That is the point -- the engine
cannot currently tell a type that does not carry revision logs from a record that lost one.

## Fix

Fixed 2026-11-05 (the maintainer decided it that day: report it, and backfill). A type's
`revision_log.required` (`MILE-159`), when a type declares it `true`, now makes a record without the
section a finding of `revision-log.change-class-required`, with the `edit-section` `add` action (`ADR-73`);
A type that does not declare it keeps the old unexamined behaviour (opt-in). The 38 ADR and RFC records that
lacked a log (15 ADRs, 23 RFCs; no spec lacked one) each gained an empty log with one structural row,
and this repository's `rfc`, `milestone`, `bug` and `waiver` types declare `required: true` beside `adr` and `spec`. The original statement of the
fix follows.

Per-type, declared, like everything else (`ADR-53`): a type states whether its records carry a
revision log, and a record of such a type without one is a finding. Types that do not are silent, by
declaration rather than by accident.

This is `ADR-14`'s append-only model having no enforcement behind it: the decision says a revision log
is how a spec changes, and a spec can drop one without the tool objecting.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** narrowing `SPEC-1` destroyed 18 dated revision entries and the rule that governs them stayed green, because it keys on a literal marker and treats its absence as nothing to check. 68 of 259 records are outside its scope and the report does not say so. | **substantive** |
> | 2026-09-19 | Deferred behind `MILE-98`. **Why:** fixable today as another hardcoded comparison in `rules.rs`, and that is the mistake this family *is* -- `PLACEHOLDER_TOKENS` transcribed by hand, `config.pointer-declaration-missing` hardcoded to one pair, `revision-log.change-class-required` keyed to a literal string. Each is a comparison written as a constant. Under the declared document model they are declarations, so building them now means building them twice and teaching the second version nothing. The gap stays open for the duration, deliberately. | **substantive** |
> | 2026-09-19 | `Blocked-on: MILE-4` declared as a field rather than described in prose. **Why:** the `bug` type did not declare `Blocked-on`, so the deferral was written into this log where no rule could see it. That is a configuration gap, not a missing rule -- `ADR-53` makes the field set a repository's declaration, and declaring it took one line. `narrative-field.stale` now reports all six of these when `MILE-98` reaches a terminal status. | **structural** |
> | 2026-09-19 | Counts reconciled and stated as a subtraction. **Why:** the record quoted 68 of 259 against `records_examined: 189` from different moments, which reads as an inconsistency. Re-measured together: 264 records, 68 without a block, 196 examined -- the arithmetic holds exactly, and the defect is that nothing in the report performs it. | **substantive** |
> | 2026-11-04 | `Blocked-on` cleared; `Status` stays `Open`. **Why:** `MILE-4` is `Done` (`section.required` reads a type's declared required sections), so the fix this bug waited on can be written; `narrative-field.stale` flagged the stale reference. | **substantive** |
> | 2026-11-04 | Triage: still reproduces on main; `Status` stays `Open`. With the `**Revision log**` block removed from `ADR-1`, `urzua check` reports nothing new; the absence is visible only as a lower `examined` count (`check_integration.rs::a_record_with_no_revision_log_is_absent_not_outside_the_population`), and `revision_log: {required: true}` gates only `urzua fix --apply`. Today 16 of 73 ADRs, 25 of 49 RFCs and 1 of 22 specs carry no block. **Why:** making a missing block a finding means choosing the default and either backfilling or waiving those records, a corpus decision. | **substantive** |
> | 2026-11-05 | Fixed. **Why:** the maintainer decided to report a missing log and backfill. `revision-log.change-class-required` now reports a record of a type that declares a log required and has none; the 38 records without one (15 ADRs, 23 RFCs) were backfilled with a single structural row, and `SPEC-2` and `SPEC-3` document the behaviour. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 4 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/revision_log.rs#a_record_of_a_type_declaring_required_false_with_no_log_is_absent_and_silent` (lines 584-602) dc3f05014091 -> ff5c3d2d3c9b. Reason: Read the diff of each pinned span since the old pin and the claim still holds. revision_log.rs#a_record_of_a_type_declaring_required_false_with_no_log_is_absent_and_silent: the test's config literal gained ..Default::default() for newly added fields, assertions unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
