---
Stable-Id: 01M6T40F6S6QC0ZN1F3KQ9Q4W5
Status: Done
Phase: '0'
Track: schema-governance
Implements: ADR-96
Depends-on: MILE-214
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/done_items_integration.rs#done_with_unresolved_items_names_each_item_and_its_cause@sha256:307c5e2229c864d88991a84f1a8f2105801771a688a553e819bda22451cc1a1c, test:rust/crates/urzua-cli/tests/done_items_integration.rs#the_variant_fails_check_under_every_fail_on_and_every_rule_level@sha256:650dabe82e006acfb1136b7cbbd0b5402352ccbcbc256702f7ad9fe5222ab598, test:rust/crates/urzua-cli/tests/done_items_integration.rs#a_rule_level_waiver_does_not_clear_it_and_still_waives_the_other_variants@sha256:e7eb941a2e2dfcb917c3547d64baff29ace8e37bae323de2061d20186027a168, test:rust/crates/urzua-cli/tests/done_items_integration.rs#some_items_resolved_is_partial_under_planned_and_fix_writes_in_progress@sha256:e97cc35a00a053a230b02301deea99beee57f430fc93bdda33364ebf6773f9fa, code:.urzua/config.yaml#record_types::milestone@sha256:133922fe07cfbd699f3a726fe846580ce470577dd7975afb44d8f52bd7b115c3, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-waiver-rules-run.yaml, code:.github/workflows/checks.yml
Non-reference-ids: WAIVER-0051
---
# 215 — Migrate the milestone corpus to Done means items and switch evidence per item on

## What

Migrate every `Done` milestone's Done means into the item list of `ADR-96`, give each item its evidence or a declared waiver, reopen any milestone whose claim is not true, and then switch `statuses.delivery.items` on for `milestone` in `.urzua/config.yaml`.

## Why

`MILE-214` builds the engine and leaves it off, so until this lands every `Done` milestone is uncovered and a falsely `Done` one stays unseen. The independent review of `ADR-96` sized the work at 143 hand-split records, most needing new locators, and asked for it to be its own milestone with a reviewer-checked list of reopens and waivers per PR.

## Done means

- Every one of the 143 `Done` milestones (the count when this was filed) carries a Done means list whose items each resolve by evidence or are waived with a declared category, or the milestone is reopened with a revision row naming the unmet item. — evidence: test:rust/crates/urzua-cli/tests/done_items_integration.rs#done_with_unresolved_items_names_each_item_and_its_cause@sha256:307c5e2229c864d88991a84f1a8f2105801771a688a553e819bda22451cc1a1c, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml
- No blanket waiver is used, and none is kept `Done` for a claim that is not true. — evidence: test:rust/crates/urzua-cli/tests/done_items_integration.rs#the_variant_fails_check_under_every_fail_on_and_every_rule_level@sha256:650dabe82e006acfb1136b7cbbd0b5402352ccbcbc256702f7ad9fe5222ab598, test:rust/crates/urzua-cli/tests/done_items_integration.rs#a_rule_level_waiver_does_not_clear_it_and_still_waives_the_other_variants@sha256:e7eb941a2e2dfcb917c3547d64baff29ace8e37bae323de2061d20186027a168
- Each migration PR lists its reopens and its waivers with the item and the reason, and for each `deferred-to` whether the target's own items cover the deferred one, and a reviewer has read each. — waived: process-outcome -- the lists are in the bodies of the migration PRs #413 to #417, each read by an independent reviewer before it merged; a review is not a file
- `.urzua/config.yaml` declares `statuses.delivery.items` for `milestone` and no other type, with the four waiver categories of `ADR-96` section 4, in the last PR only, once `urzua check` is clean over every milestone. — evidence: code:.urzua/config.yaml#record_types::milestone@sha256:133922fe07cfbd699f3a726fe846580ce470577dd7975afb44d8f52bd7b115c3
- `WAIVER-0051` is removed and `MILE-165`, rewritten into items, computes `InProgress` from the engine. — evidence: test:rust/crates/urzua-cli/tests/done_items_integration.rs#some_items_resolved_is_partial_under_planned_and_fix_writes_in_progress@sha256:e97cc35a00a053a230b02301deea99beee57f430fc93bdda33364ebf6773f9fa, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-waiver-rules-run.yaml, test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml
- `urzua check` and `urzua check --base origin/main` report 0 unwaived findings, including no `DoneWithUnresolvedItems` and no `status.waived-item-stale`. — evidence: test:rust/crates/urzua-cli/tests/acceptance/corpora/corpus-zero/expectations/own-config-status-rules-run.yaml, code:.github/workflows/checks.yml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-07 | Filed. **Why:** split out of `MILE-214` on the independent review of `ADR-96`, so the engine can land before the corpus is migrated and switched on. | **substantive** |
> | 2026-11-07 | Cleared `Blocked-on`: `MILE-214` is Done and `ADR-96` is Accepted. **Why:** the wait it named is over; `Depends-on` still orders the two. | **structural** |
> | 2026-11-07 | `Status` Planned -> Done; `Realized-by` added; Done means rewritten into `ADR-96` items with per-item evidence. **Why:** the migration is delivered: #413 to #417 rewrote every milestone's Done means into items, `.urzua/config.yaml` now declares `statuses.delivery.items` for `milestone`, `WAIVER-0051` is removed, and the engine tests of `MILE-214` and the corpus-zero cases of this repository hold the result. The switch-on moved 22 `Done` milestones to `InProgress`, `MILE-14` and `MILE-101` (`Planned`) to `InProgress`, and reopened nothing else; the 14 pending changesets that listed a reopened milestone in `closes` no longer list it. | **substantive** |
> | 2026-11-07 | `Depends-on: MILE-214` removed; `Status` unchanged. **Why:** `MILE-214` is `InProgress` from its Done means items (one item, running the acceptance corpus through `explain`, has no evidence) and a blocked milestone cannot claim progress (`sequence.status-agreement`). The ordering the dependency recorded, the engine landing before the switch-on, is met: the engine merged in #411 and every engine test this milestone cites passes. The dependency was real as an ordering and that ordering is met; `Depends-on` is a whole-milestone gate, and `MILE-214`'s open item concerns `explain`, which the migration does not use. Keeping it was tried: `Done` is refused by `sequence.status-agreement` and `InProgress` by `status.delivery-agreement`, because every item resolves. Restore `Depends-on: MILE-214` once its item 30 has evidence. | **structural** |
> | 2026-11-07 | `Depends-on: MILE-214` restored and `Status` Done -> InProgress. **Why:** the `ADR-96` amendment of 2026-11-07 (accepted by the maintainer) makes dependencies cap the status: every item resolves, but `MILE-214` is unfinished (`InProgress`, its item 30 has no evidence), so the status computes to `InProgress`, which both `status.delivery-agreement` and `sequence.status-agreement` accept. The removal of 2026-11-07 named this restore: `MILE-214`'s item 30 still has no evidence. | **substantive** |
> | 2026-11-07 | Tool-authored (by beauwilliams, identity: gh-verified, actor: unspecified): recomputed `Status` from `the Done means items (5 of 6 items resolved, 1 waived) computes to Implemented` -- stated `InProgress` disagreed with the computed value `Done`. | **structural** |
