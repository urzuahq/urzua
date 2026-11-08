---
Status: Done
Stable-Id: 01M1Y5JD3YB2532B114C3ZC87P
Phase: '0'
Track: header-format
Implements: ADR-33
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:6c671b5f6ed730ac5389ed12c9bda023c7bd38ee02f74bb087e520e45f0a6aa4, test:rust/crates/urzua-cli/tests/check_integration.rs#init_then_check_names_a_pre_existing_blockquote_record_as_an_unsupported_form@sha256:1b223594d7badb7448e36b5647e11808f59aa675ca9bdfd2bb694aedff0cce4c
---
# 2 — Add header.deprecated-shape rule and flip init's default to yaml-frontmatter

## What

A non-blocking check-rule warning when a record type's configured shape isn't yaml-frontmatter, and init proposing yaml-frontmatter for any newly-adopted type.

## Why

ADR-33 decided the deprecation; this is the mechanism that actually makes it visible and steers new adoptions, without breaking anyone still on blockquote/bold-list.

## Done means

- `urzua init` in adopt mode proposes `header_shape: yaml-frontmatter` for every type it proposes, whatever shape the corpus's records were detected in. — evidence: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:6c671b5f6ed730ac5389ed12c9bda023c7bd38ee02f74bb087e520e45f0a6aa4, test:rust/crates/urzua-cli/tests/check_integration.rs#init_then_check_names_a_pre_existing_blockquote_record_as_an_unsupported_form@sha256:1b223594d7badb7448e36b5647e11808f59aa675ca9bdfd2bb694aedff0cce4c
- `urzua init` warns that records not in that shape will be unreadable until rewritten. The non-blocking `header.deprecated-shape` rule this milestone also built was retired with the shapes it warned about by `MILE-85`, so what remains true is the `init` default. — evidence: code:rust/crates/urzua-cli/src/commands/init.rs@sha256:6c671b5f6ed730ac5389ed12c9bda023c7bd38ee02f74bb087e520e45f0a6aa4, test:rust/crates/urzua-cli/tests/check_integration.rs#init_then_check_names_a_pre_existing_blockquote_record_as_an_unsupported_form@sha256:1b223594d7badb7448e36b5647e11808f59aa675ca9bdfd2bb694aedff0cce4c

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Cleared `Blocked-on` (was `BUG-3`, `Status: Fixed` for a while, caught by `blocked-on.stale` -- MILE-83 -- immediately after `Blocked-on` moved from prose into a checked header field). **Why:** the blocker had silently resolved and nobody had revisited this milestone since; found live during backlog triage, confirmed by the new rule firing on this exact case before this fix. | **substantive** |
> | 2026-09-08 | `Status: Done`. Built `header.deprecated-shape` (config-level, one warning per non-yaml-frontmatter type) and `init`'s adopt-mode default (`header_shape = "yaml-frontmatter"` on every proposed type, regardless of the corpus's detected shape). Verified: the planted-violation tests in `rules.rs`, and `init_then_check_flags_a_pre_existing_blockquote_record` observing the real, ADR-33-accepted consequence of that default against a still-blockquote corpus. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Partly superseded by `MILE-85`; `Status` stays Done. **Why:** the closure audit of #234 swept the Done milestones for later removals. `header.deprecated-shape` was deleted by `MILE-85` because it can no longer fire once the deprecated shapes are unsupported; `init` still defaults to `yaml-frontmatter`. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 1 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/init.rs@sha256:efe3f3739082 Reason: Pinned the files as they are now after checking each against the record it evidences: sequence.rs still reads roadmap.pairs as one graph (MILE-192), waiver.rs and config.rs still read the permanent declaration and the sixteen WAIVER records and the waiver template still exist as the migration the milestone describes (MILE-193), config.rs, config.yaml, the Makefile, checks.yml, MILE-189 and generate-rule-table.py still carry the info level (MILE-194), harness/run.rs and config.rs still offer block_on all (MILE-195), config.rs and rules/roadmap.rs still read work_types (MILE-196), config.rs and options.rs still read verb_claims (MILE-197), the harness modules and the changeset still install and run the engine hooks (MILE-198), bundle.rs, the bundled and repo-local command files and urzua-record.sh still draft every record type and the lifecycle commands (MILE-199), init.rs still proposes yaml-frontmatter (MILE-2), and authority.rs, harness/gate.rs, rules/decider.rs and the urzua-gate action scripts still enforce who may accept a record (MILE-200). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` efe3f3739082 -> 08dfadd3cae6. Reason: Read the diff of each pinned file since the old pin and the claim still holds. init.rs: only the doc comment on run changed, to say the config a dry run proposes is byte-identical to the file the real run writes and that the reports differ in dry_run, written and the echoed config_yaml; no code changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 08dfadd3cae6 -> 5f376bdd1b6d. Reason: init.rs now words the header-shape notice truthfully (check reports unreadable headers only when required_fields declares them), excludes README.md for a directory holding one, and carries the merged doc-comment edit from main; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/init.rs` 5f376bdd1b6d -> 6c671b5f6ed7. Reason: split init::run into propose_record_types and write_config, and render_config_yaml into proposed_types_mapping and proposed_rules_mapping (init.rs); behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
