---
Status: InProgress
Stable-Id: 01M1YNNAAMTDJMEHET3W5XK1QE
Phase: '0'
Track: roadmap-tracking
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_emits_json_not_plain_text_lines@sha256:f1206a0ad9ab58de9de017375f35f62be7cca473c3fca30d6ed17d4e15d9f9bf, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_missing_config_as_exit_2@sha256:8b081b4cb7cf92bff8151fbe0044e59f9992c1965e8c99be0f6c09979cda2e73, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_an_invalid_record_type_prefix_or_dir@sha256:0cf80deb45aac8cd65dde8413b0f55e9df06f475715d63a72ba8add516c29bf8, test:rust/crates/urzua-cli/tests/check_integration.rs#ci_wired_finds_the_invocation_in_any_workflow_not_just_ci_yml@sha256:77e8117a378c2069342a919b65f3aeb418f559bdfcb26794ea5b94feae8007ce, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#doctor_reads_the_repositorys_own_config_and_reports_the_unwired_hook_as_a_warning@sha256:7912fc15a4c3d9b378891782bbdefee6c16e7044a8b4e162f7a22b1eca71c54e, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_errors_when_no_rule_is_enabled@sha256:55ff07249fcf04268bd95ac62fb14956c513ee697238ba22bbe53cc815831372, code:Makefile, code:.git-hooks/pre-commit, code:.git-hooks/pre-push, code:.github/workflows/checks.yml
---
# 58 — Build urzua doctor

## What

`urzua doctor`: reports on the tool's own configuration and invocation health (does a config exist, does it parse, is `check` actually wired into CI) rather than on record content.

## Why

A checker that's correct but never invoked is indistinguishable from no checker at all -- doctor answers "is this actually running," a different question than "did this find anything."

## Done means

- `urzua doctor` reports on the tool's own health rather than on record content. — evidence: code:rust/crates/urzua-cli/src/commands/doctor.rs, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_emits_json_not_plain_text_lines@sha256:f1206a0ad9ab58de9de017375f35f62be7cca473c3fca30d6ed17d4e15d9f9bf
- `urzua doctor` reports whether a config exists. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_missing_config_as_exit_2@sha256:8b081b4cb7cf92bff8151fbe0044e59f9992c1965e8c99be0f6c09979cda2e73
- `urzua doctor` reports whether the config parses. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_an_invalid_record_type_prefix_or_dir@sha256:0cf80deb45aac8cd65dde8413b0f55e9df06f475715d63a72ba8add516c29bf8
- `urzua doctor` reports whether a record type declares what the rules need (such as required fields). — evidence: code:rust/crates/urzua-cli/src/commands/doctor.rs
- `urzua doctor` reports whether `check` is wired into CI and the local git hooks. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#ci_wired_finds_the_invocation_in_any_workflow_not_just_ci_yml@sha256:77e8117a378c2069342a919b65f3aeb418f559bdfcb26794ea5b94feae8007ce, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs#doctor_reads_the_repositorys_own_config_and_reports_the_unwired_hook_as_a_warning@sha256:7912fc15a4c3d9b378891782bbdefee6c16e7044a8b4e162f7a22b1eca71c54e
- It exits non-zero on an unhealthy setup. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_errors_when_no_rule_is_enabled@sha256:55ff07249fcf04268bd95ac62fb14956c513ee697238ba22bbe53cc815831372, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_reports_missing_config_as_exit_2@sha256:8b081b4cb7cf92bff8151fbe0044e59f9992c1965e8c99be0f6c09979cda2e73
- Each check is covered in the integration tests.
- It runs clean on urzua's own repository. — evidence: code:Makefile, code:.git-hooks/pre-commit, code:.git-hooks/pre-push, code:.github/workflows/checks.yml

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 7. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
