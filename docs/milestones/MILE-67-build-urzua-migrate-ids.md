---
Status: Done
Stable-Id: 01M1YNNF8F2GD7BAQVX9E5Q7K5
Phase: '0'
Track: roadmap-tracking
Implements: ADR-3, ADR-21
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner@sha256:7bc452b4fb4d5f235f5104b4a7d1d8e513decf3eb07532a75caeea05fd93201f, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:2d0a089cfa4c56519cc448c2fdbf0f1c048828ac32b60aaeae778470ebd56bce, test:rust/crates/urzua-id/src/lib.rs#two_generated_ids_never_collide@sha256:027c5dfca14618d24c06d1026243457cede9f119c70c2f688cc952d1b873ec2a, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_reports_per_file_outcomes_as_structured_json@sha256:1b28d251ff67a79e72a91daac97cd64243db7aaa8711f50fcd19ddb0bfeaddd8, spec:docs/specs/SPEC-14-urzua-migrate.md
---
# 67 — Build urzua migrate ids

## What

`urzua migrate ids`: backfills a collision-free `Stable-Id` (ULID) into every record lacking one, without touching the filename or any cross-reference -- the display number stays exactly what it already is. Dry-run by default.

## Why

ADR-3's stable/display split only matters if every record actually has a stable ID -- this is what makes the split real for a corpus that predates it, without forcing a disruptive renumbering to get there.

## Done means

- `urzua migrate ids` adds a collision-free ULID `Stable-Id` to every record lacking one. — evidence: code:rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner@sha256:7bc452b4fb4d5f235f5104b4a7d1d8e513decf3eb07532a75caeea05fd93201f, test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:2d0a089cfa4c56519cc448c2fdbf0f1c048828ac32b60aaeae778470ebd56bce, test:rust/crates/urzua-id/src/lib.rs#two_generated_ids_never_collide@sha256:027c5dfca14618d24c06d1026243457cede9f119c70c2f688cc952d1b873ec2a
- `migrate ids` does not touch a filename or a cross-reference. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_preserves_crlf_and_the_absent_trailing_newline@sha256:2d0a089cfa4c56519cc448c2fdbf0f1c048828ac32b60aaeae778470ebd56bce, code:rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner@sha256:7bc452b4fb4d5f235f5104b4a7d1d8e513decf3eb07532a75caeea05fd93201f
- `migrate ids` is a dry run unless applied. — evidence: code:rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner@sha256:7bc452b4fb4d5f235f5104b4a7d1d8e513decf3eb07532a75caeea05fd93201f
- A second applied `migrate ids` run changes nothing. — evidence: code:rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner@sha256:7bc452b4fb4d5f235f5104b4a7d1d8e513decf3eb07532a75caeea05fd93201f
- The command is covered in `check_integration.rs` and specified in `SPEC-14`. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#migrate_ids_apply_reports_per_file_outcomes_as_structured_json@sha256:1b28d251ff67a79e72a91daac97cd64243db7aaa8711f50fcd19ddb0bfeaddd8, spec:docs/specs/SPEC-14-urzua-migrate.md

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-05 | Backfilled: shipped before milestone tracking existed. | **structural** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner` (lines 84-134) f51f9d55ae0b -> 3e27c52ab497. Reason: split run_ids_inner, moving per-record work into backfill_stable_id (migrate.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner` (lines 88-151) 3e27c52ab497 -> 3bc46f27a594. Reason: migrate.rs: migrate ids now reports a written count and skips a header_shape none record with a reason and a warning notice; the backfill of headed records is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner` (lines 95-160) 3bc46f27a594 -> 7706e4ce4e3d. Reason: migrate.rs: migrate ids reports written, skips a header_shape none record with a reason and notice, and reports not-run exit 2 when it could write none **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/migrate.rs#run_ids_inner` (lines 96-162) 7706e4ce4e3d -> 7bc452b4fb4d. Reason: Read the diff of rust/crates/urzua-cli/src/commands/migrate.rs against main. rust/crates/urzua-cli/src/commands/migrate.rs: Failure now carries a non-optional kind and has no conversion from a bare message, so a refusal states its kind where it is made; its argument-free steps are tagged environment, its configuration reads config. The claims each record cites are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
