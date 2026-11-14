---
Stable-Id: 01M2MF5T28P4GAQRK2VYBDFZRE
Status: Fixed
Found-in: 'a review pass over urzua-core while fixing BUG-23 -- `is_active` compares `Expires` lexically without ever checking it is a date'
Realized-by: code:rust/crates/urzua-core/src/waiver.rs@sha256:2ac7aae607817d5c346a0e6a9d9c38254a71ccd3dd4a083324ba4819f285c53e, test:rust/crates/urzua-core/src/waiver.rs#an_unparseable_expiry_does_not_suppress_observed_failing@sha256:9613f7d573d3a2e759d2daa6c6da06ee7ffd1d6116ef6535cc91c27d7de21346
---
# 31 — A waiver with an unparseable Expires value never expires

## What was wrong

`Waiver::is_active` decided expiry with a string comparison:

```rust
Some(expires) => today <= expires.as_str(),
```

`Expires` was never parsed or validated. ASCII digits sort below letters, so `"2026-09-16" <= "soon"`
is `true` — and stays true on every future date. `Expires: soon`, `Expires: TBD`, `Expires: Q4`, and
a mistyped `Expires: 2026-9-16` all produced a waiver that **can never expire**.

The method's own doc comment asserted the invariant it did not enforce: *"ISO 8601 dates compare
correctly as plain strings."* True once both sides are known dates, which nothing established.

This is the one direction a suppression mechanism must not fail. The module doc argues a malformed
waiver "fails toward more findings, not fewer" — correct for a missing `Rule` or `Scope`, which
`load_waivers` drops via `filter_map`, and exactly inverted here.

## Why nothing caught it

`docs/waiver/` is empty — this repo has never written a waiver, so the path has never run outside its
own unit tests, and those only ever passed well-formed dates. `field.quality` does not reach it
either: that rule examines a type's `required_fields`, and `Expires` is optional for `waiver`, so
even `tbd` — already in `PLACEHOLDER_TOKENS` — is never classified here.

A mechanism whose only consumer is its own tests is not exercised, and a suppression mechanism that
is never exercised fails in the expensive direction the first time someone uses it.

## The fix

`is_active` requires `YYYY-MM-DD` before trusting the comparison; anything else expires immediately.
Calendar validity is deliberately unchecked — a 31st of February still orders correctly against a
real date, which is all this comparison asks of it.

**Deliberately not fixed here:** a malformed expiry now suppresses nothing, but nothing reports that
it was malformed either. Surfacing it needs a `waiver.*` rule, which would mean a new rule id, a
`SPEC-2` row and a `README` row — scope this change does not carry. Filed as the remaining half.

## References

- `rust/crates/urzua-core/src/waiver.rs` -- `is_active`, `is_iso_date`.
- RFC-15 -- waivers as time-boxed first-class records; §3 is the rule this restores.
- ADR-9 -- a detector fails open, a prohibition fails closed. A waiver is a prohibition on reporting.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change, minus the reporting half. **Why:** latent -- there are no waiver records in this corpus, so this has never fired and the original framing of it as urgent was wrong. It is still the only place in the tool where failing open removes findings rather than adding them, which is why it was worth fixing before anyone writes the first waiver. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs#an_unparseable_expiry_does_not_suppress_observed_failing` (lines 483-514) 7db66c82a4fa -> 64c2268b8e2d. Reason: Read the diff of each pinned span since the old pin and the claim still holds. waiver.rs#an_unparseable_expiry_does_not_suppress_observed_failing: the test loops over more invalid expiry spellings (2026-99-99, 2026-19-01, 2026-02-30, 2026-13-01 and 2025-02-29 beside the earlier four) and its fixture struct gained unmet_states and permanence fields, and it still asserts every unparseable expiry fails to suppress. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/waiver.rs@sha256:b8d83e370c42 Reason: Pinned the files as they are now after checking each against the record it evidences: publish-release.yml still builds and uploads the archives itself (BUG-28), new_record.rs still renders only the configured YAML header and new_integration.rs still holds the test that a blockquote template does not override it (BUG-3), waiver.rs is_active still parses Expires as a date (BUG-31), prepare-release.yml still hands knope's nothing-to-release exit to release-guard.sh instead of failing (BUG-33), ci.yml's changeset job still reads the label live (BUG-34), knope.toml still releases from changesets only (BUG-35), main.rs and check_integration.rs still emit doctor as JSON (BUG-4), release-guard.test.sh still plants an unmatched-package fragment and asserts a nonzero exit (BUG-48), field_state.rs still carries the retired placeholder tokens (BUG-5), and generate-rule-table.py and the Makefile rule-table-check target still compare SPEC-2's rule table with the shipped rule set (BUG-52). BUG-29 and BUG-30 are not in this batch because the filename.title-consistency code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs` b8d83e370c42 -> 748f534e72aa. Reason: Read the diff of each re-pinned file against main. waiver.rs: added SuppressionType::field_state, and load_suppressions now calls it; the classification it holds is the code that was inline in load_suppressions, moved unchanged, so which waivers take effect is unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/waiver.rs` 748f534e72aa -> 2ac7aae60781, `rust/crates/urzua-core/src/waiver.rs#an_unparseable_expiry_does_not_suppress_observed_failing` (lines 569-602) 64c2268b8e2d -> 9613f7d573d3. Reason: Read the diff of each pinned file against main and the claim still holds. waiver.rs: Suppression gained variant and key_path (read from the Variant and Key-path header fields), names and is_narrowed, and covers is now names plus waivability; matching on rule and scope is unchanged for a waiver naming neither field; the test's Suppression literal gained variant: None and key_path: None; its assertions are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
