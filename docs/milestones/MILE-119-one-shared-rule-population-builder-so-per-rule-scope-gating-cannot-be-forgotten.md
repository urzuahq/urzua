---
Stable-Id: 01M6CXZVGSSA8BEM1D0QTQKFE5
Status: Done
Phase: '0'
Track: governance-process
Realized-by: code:rust/crates/urzua-core/src/rules/shared.rs#scoped_census_records@sha256:2775fec529d50ccc0144d09afbad78e450e94b8a2fd8ff65c6a5523965ebc02a, test:rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope@sha256:88bdb8265ec6c026b3b4e290afa9a6cb22a845bdd320b29fb97600f5e1e6f139
---
# 119 — One shared rule-population builder, so per-rule scope gating cannot be forgotten

## What

Route every rule in `rust/crates/urzua-core/src/rules.rs` through one shared population builder that
applies scope gating once: a `header_shape: none` type, an unreadable header, and the other
out-of-scope cases are classified `OutOfScope` there, not re-derived inside each rule. Add a
table-driven test that iterates over every shipped rule (the registered rule set, not a hand-picked
list) on a fixture type declaring `header_shape: none` and asserts each reports `OutOfScope`, or
carries a documented exemption, for that type, so a future rule that skips the builder fails the test
without anyone remembering to write one.

## Why

Each rule builds its own population and must separately remember to gate on `has_no_header()` before
treating an absent region as unreadable. The same defect has been fixed one rule at a time: `BUG-98`
(`header.required-fields` and the shape itself), `BUG-124` (a config rule with no `none` exemption),
`BUG-137` (`migrate::schema_report`), `BUG-142` (`field.quality`, `field.pending`, and
`header.required-fields`' slot loop) and `BUG-147` (four more rules, on an open PR). `BUG-142` and
`BUG-147` each say in "Why nothing caught it" that the prior fix was scoped to the rules a review pass
happened to look at. Review rounds 9 to 26 each found three or four bugs with no downward trend, the
signature of instance fixes applied to a mechanism that keeps producing the defect. `AGENTS.md`'s
review loop, step 4, says to fix the mechanism. `BUG-125` is the precedent: the same collapse of distinct
states found in three rules was fixed once in the shared mechanism, not three times at the call sites.
`BUG-128` is adjacent (config-lint coverage of `none` types) and is not counted as an instance.

## Done means

- Every header-reading rule is gated through one shared builder, `scoped_census_records` in `rules/shared.rs`. — evidence: code:rust/crates/urzua-core/src/rules/shared.rs#scoped_census_records@sha256:2775fec529d50ccc0144d09afbad78e450e94b8a2fd8ff65c6a5523965ebc02a
- That builder classifies a `header_shape: none` type, an unreadable header and the other out-of-scope cases as `OutOfScope` once. — evidence: code:rust/crates/urzua-core/src/rules/shared.rs#scoped_census_records@sha256:2775fec529d50ccc0144d09afbad78e450e94b8a2fd8ff65c6a5523965ebc02a, test:rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope@sha256:88bdb8265ec6c026b3b4e290afa9a6cb22a845bdd320b29fb97600f5e1e6f139
- A table-driven test iterates every shipped rule over a `header_shape: none` type and fails for a rule that examines it. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope@sha256:88bdb8265ec6c026b3b4e290afa9a6cb22a845bdd320b29fb97600f5e1e6f139

## References

- `BUG-98`, `BUG-124`, `BUG-137`, `BUG-142`, `BUG-147` -- the per-rule instances.
- `BUG-125` -- precedent for fixing a recurring class once in the shared mechanism.
- `ADR-50` -- `header_shape: none`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** `header_shape: none` gating has been fixed per rule five times and each fix covered only the rules a pass looked at; filed to fix the population mechanism once, with a test that covers every rule. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #136: every header-reading rule is gated through `scoped_census_records`, and a table-driven test iterates every shipped rule over a `header_shape: none` type. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#every_header_reading_rule_reports_a_header_none_type_as_out_of_scope` (lines 3342-3455) 60033116789c -> 88bdb8265ec6. Reason: Read the diff of the pinned test: every_header_reading_rule_reports_a_header_none_type_as_out_of_scope gained one config arm giving embodiment.claim-expired the periods and anchor_field it needs to load, and its assertions are unchanged, so the test still requires every non-exempt rule in the table, the two new expiry rules included, to count a header_shape none type as out of scope. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
