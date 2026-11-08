---
Stable-Id: 01M6R3NK4PF53R3RGYQVBWGATM
Status: Accepted
Embodiment: Verified
Realized-by: code:.urzua/config.yaml#record_types::bug@sha256:db5d407b1b71173ea2819d4b2213de057659bb319a8cffc32edee485a73fe44c, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#a_bug_needs_found_in_and_a_what_was_wrong_section@sha256:83080eb2059b6c6eb69353b0a8730fa705e8a160c25134f84c7c16c18ee206bf, test:rust/crates/urzua-cli/tests/own_config_integration.rs#a_bug_open_with_test_evidence_is_a_finding_and_the_other_shapes_are_not@sha256:6cb8690e1d1ec8c45d69336b1c3f2e5c752c26f181d869bc2e77fc02e71d0e6b
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-34
---
# 35 — `bug`: a configured record type requiring a named regression test

## Context

`check`'s `paths` argument was found, live, to do nothing: it located the repo root and was then
silently discarded, so `check docs/adr/` and `check docs/` always returned identical results.
Nothing caught this because nothing had ever exercised a genuinely narrower path — every existing
test called `check docs/`. This is exactly the "no check runs its own regex... no test caught it
because no real record ever needed the fallback" defect class this project's own practice already
names, discovered here about its own CLI argument rather than a rule.

The immediate question was whether to track this as a `milestone` (`Track: bug-fix`) or as its own
type. Checked against what a bug genuinely needs that a milestone doesn't: a milestone is
*prospective* (`Status`, `Phase`, `Track` — planned work moving toward done); a bug is
*retrospective* (what was actually wrong, how it was found, and — the one real governance gain — a
named, checkable pointer to the test that proves it's fixed). Forcing a bug into milestone's schema
loses that pointer or bolts it on as an optional field nothing requires.

## Decision

In the context of a real defect needing a home, facing a schema question milestone's own fields
don't answer well, we decided: **`bug` is a separate configured record type — zero `urzua-core`
changes, same footprint as `milestone` and `waiver`.**

```toml
[record_types.bug]
dir = "docs/bugs"
required_fields = ["Status", "Found-in", "Regression-test"]
```

- `Status`: `Open | Fixed | WontFix`.
- `Found-in`: how/where it was discovered — free text, since this is inherently a narrative
  (`"check docs/adr/ vs check docs/ compared by hand"`), not a resolvable reference.
- `Regression-test`: the specific test name/path proving the fix — **required**, not optional. A
  bug record with no regression test is exactly a claim nobody verified, which is the failure mode
  this whole field exists to make impossible to state by accident.
- `Realized-by`: reused, generic — points at the actual fix commit/function, same as any other
  type's evidence field.

## Reversibility

Fully additive: one config entry, one template, one new directory.

## Consequences

- A `bug` record with `Status: Fixed` but a blank/placeholder `Regression-test` is a
  `field.quality` finding already, for free — no new rule needed, since required-field quality
  checking is already generic.
- This ADR's own `Realized-by` doubles as the first real `bug` record's evidence, and
  `docs/bugs/0001-*.md` names this exact defect, required-field-complete, as the first real
  instance of the type.

## Amendment (2026-11-04): the `bug` type declares five statuses

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

The Decision gives `Status` as `Open | Fixed | WontFix`. The `bug` type in `.urzua/config.yaml` now
declares five: `Open`, `Fixed`, `WontFix`, `Not a bug` and `Rejected`. The required fields
(`Status`, `Found-in`, `Regression-test`) and the rest of the Decision are unchanged. The declared
list in the config is the authority; this text does not enumerate it again.

## Amendment (2026-11-05): the named test is carried by `Realized-by`, and `Regression-test` is retired

*Status of this amendment: accepted 2026-11-05 by the maintainer (drafted 2026-11-05 at the maintainer's request). `Status` stays `Accepted` and the Decision text above is unchanged.*

The Decision requires a bug to name the test that proves its fix, and says why: a bug with no
regression test is a claim nobody verified. That requirement still holds. What changes is where it is
written. `ADR-84` makes `Realized-by` the one evidence field of every record type and holds a record's
`Status` to it, so a bug now names its test as a `test:` locator in `Realized-by`, and the
`Regression-test` field is retired: it is no longer in the `bug` type's `required_fields`, the
template or `SPEC-9`.

What a reader of the Decision should take from this:

- A `Fixed` bug with a `test:` locator computes to `Verified`; a `Fixed` bug with no evidence at all is
  a `status.delivery-agreement` finding, because the `bug` type declares `Not started` as an
  unevidenced tier. The check that a named test exists is no longer `field.quality` on a blank
  `Regression-test`; it is `embodiment.locator-exists` and the evidence pin on the locator.
- A bug that has no test because none fits (a records, documentation, workflow or performance change)
  is not made to cite one; it is recorded as a waiver (`ADR-11`), so the exception stays visible.
- The `Realized-by` line in the Decision, which points "at the actual fix commit/function", is
  unchanged and now also carries the test locators.

The `Context`, the `Reversibility` section and the first Consequence describe the field as it stood
and are left as written.

## References

- ADR-34 — the `milestone` type this is a sibling of, and the reasoning this ADR follows.
- `rust/crates/urzua-cli/src/main.rs` — `scope_to_requested_paths`, the fix.
- `rust/crates/urzua-cli/tests/check_integration.rs` —
  `check_scopes_to_the_requested_path_not_the_whole_corpus`, the regression test.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 784fa758e. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `.urzua/config.yaml` e185c4da7b1f -> 8bc61e85bdcc, `rust/crates/urzua-cli/tests/check_integration.rs` 288a08b371a8 -> b668d62f2f4a. Reason: Re-verified: the bug type still requires Status, Found-in and Regression-test; config.yaml growth is unrelated rule options and types. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-04 | Amendment added: the `bug` type declares five statuses, not the three the Decision lists. **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision is unchanged. | **substantive** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Narrowed to the item the decision concerns; re-pin approved by the maintainer's 2026-11-04 drift decision (`ADR-74`, `MILE-185`): `config.yaml` to `record_types::bug` (lines 247-295). The test locator `check_integration.rs`, which has no test of the `Regression-test` requirement, is replaced, not narrowed, by `own_config_integration.rs#a_fixed_bug_declaring_no_regression_test_is_a_finding_and_an_open_one_is_not` (lines 122-167). **Why:** an unrelated edit to a volatile file must not stale this pin again; the spans were chosen by the author of this change and not reviewed line by line by the maintainer. | **structural** |
> | 2026-11-05 | Re-baselined (BUG-50, attested by this PR's review): re-pinned `.urzua/config.yaml#record_types::bug` 0f500a61157f -> ae2814c161f8. Reason: BUG-50 added one `revision_log: {marker, required: true}` declaration to the bug type; its Found-in, Regression-test and section requirements this record cites are unchanged **Why:** the pinned evidence changed and the PR review is the human re-verification that the claim still holds. | **structural** |
> | 2026-11-05 | `Realized-by` test locator re-pointed: `own_config_integration.rs#a_fixed_bug_declaring_no_regression_test_is_a_finding_and_an_open_one_is_not` no longer exists, replaced by `record_types_own_config_integration.rs#a_bug_needs_found_in_and_a_what_was_wrong_section` (the type's required fields) and `own_config_integration.rs#a_bug_open_with_test_evidence_is_a_finding_and_the_other_shapes_are_not` (the type's evidence). The Decision text is unchanged; its `Regression-test` field is retired and its named test is carried by `Realized-by` locators (`ADR-84`, and the amendment of this date). **Why:** `ADR-84`'s repository migration retired the field and rewrote the test that exercised it. | **structural** |
> | 2026-11-05 | Amendment accepted: the requirement for a named test still holds, now carried by `Realized-by` under `ADR-84`, and `Regression-test` is retired. **Why:** Maintainer decision, 2026-11-05. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `.urzua/config.yaml#record_types::bug` (lines 274-332) ae2814c161f8 -> 908086d76c33. Reason: The bug type's declaration changed: Regression-test is removed from required_fields and from statuses.require, and statuses.delivery is added (delivered_tiers Verified and Implemented, unevidenced_tiers Not started, not_done Open, set_status Fixed, back_pointer_evidence false). The type still requires Status and Found-in. It no longer requires a Regression-test field; the requirement for a named test is now carried by Realized-by under ADR-84, as the amendment of 2026-11-05 records **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-cli/tests/own_config_integration.rs#a_bug_open_with_test_evidence_is_a_finding_and_the_other_shapes_are_not` (lines 122-160) d256a87ded76 -> 6cb8690e1d1e. Reason: own_config_integration.rs: a_bug_open_with_test_evidence_is_a_finding_and_the_other_shapes_are_not no longer lists a Fixed bug with no evidence among the shapes that raise no finding, and now asserts that shape reports status.delivery-agreement, since the bug type declares Not started as an unevidenced tier; the Open-with-evidence assertion it cites is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-06 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::bug` (lines 307-365) 908086d76c33 -> 8e624908f1e8. Reason: The bug record type in .urzua/config.yaml now lists Stable-Id under required_fields (moved from known_fields); the Found-in requirement and regression-test evidence this decision cites are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `.urzua/config.yaml#record_types::bug` (lines 351-415) 8e624908f1e8 -> e48fcdc3c068. Reason: The bug type's revision_log declaration in .urzua/config.yaml now also declares classes, class_column, date_column and tool_class with today's values (MILE-213, ADR-94); the type's other keys are unchanged and behaviour is identical. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::bug` (lines 370-435) e48fcdc3c068 -> db5d407b1b71. Reason: Read the diff of each pinned span since the old pin and the claim still holds. config.yaml#record_types::bug: this change added Last-audited to the type's known_fields, the rest of the declaration is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
