---
Stable-Id: 01M371W3V40P7MTF96QVDXA81N
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 25"
Realized-by: test:rust/crates/urzua-core/src/migrate.rs#a_header_none_type_is_not_reported_as_unreadable_observed_failing@sha256:cd5555a829f94f2c058356b2d24d2528cc94781d844b88bd963df66690f7d292
---
# 137 — migrate schema-report fails to gate on ADR-50's header-shape-none types

## What was wrong

`urzua_core::migrate::schema_report` calls `record.header.is_unreadable()` directly, with no check for
whether the record's type declares `header_shape: none` (`ADR-50`) first — unlike every other
`is_unreadable()` call site in `rules.rs` (e.g. `header_required_fields`), which all gate on
`RecordTypeConfig::has_no_header()` before treating an absent region as a parse failure.

A `none`-shaped type's records have no header at all, so `region` is always `None` by construction and
`is_unreadable()` is always `true` for them — not a defect. Running `urzua migrate schema --report
<field>` on a corpus with such a type reported the entire type as `Unreadable` with a spurious "header
did not parse" notice, and excluded it from the preview, even though `config.header-none-has-no-required-fields`
already forbids that type from ever declaring the candidate field in the first place.

## Why nothing caught it

`schema_report` did not take `config: &Config` at all, so it had no way to check `has_no_header()` even
if someone had thought to. No existing test used a `header_shape: none` type.

## References

- `header_required_fields`'s `has_no_header()` gate (`rules.rs`) — the established pattern this
  function now follows.
- `BUG-98` — the earlier instance of the same defect class (`ADR-50` unimplemented/ungated) in a
  different rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test observed failing (reported `Unreadable` instead of `OutOfScope`) before the fix. `schema_report` now takes `config: &Config`; its one production call site and four test call sites updated. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/migrate.rs#a_header_none_type_is_not_reported_as_unreadable_observed_failing` (lines 202-229) 36852057e2e6 -> cd5555a829f9. Reason: Read the diff of each pinned span since the old pin and the claim still holds. migrate.rs#a_header_none_type_is_not_reported_as_unreadable_observed_failing: schema_report now takes a templates argument that the test passes as Default::default(), assertions unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
