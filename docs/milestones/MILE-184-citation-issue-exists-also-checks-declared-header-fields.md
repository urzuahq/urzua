---
Stable-Id: 01M6K68092WYW68GYMDXNE7SG8
Status: Done
Phase: '0'
Track: schema-governance
Implements: SPEC-1
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2, code:rust/crates/urzua-core/src/rules/citation.rs@sha256:a1c156d0395a2bcad18f5763b56e31b48d622a534b15cf0795df9b08cfd4758a, test:rust/crates/urzua-core/src/config.rs#issue_refs_fields_must_be_declared_header_fields_and_never_blank@sha256:3c1d9033c06d0894d26493fddc21ead263fc5f9fd3141161aae4dbb825327fb7, code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089
---
# 184 — citation.issue-exists also checks declared header fields

## What

`citation.issue-exists` reads the header fields a repository names in `issue_refs.fields` (for example `Found-in`) for issue and pull request citations, as well as the declared reference sections. The option is off unless declared, and a field name no record type declares is refused at load.

## Why

The rule read only `reference_sections`, so a wrong PR number in a header field such as a BUG's `Found-in` went unchecked, though that is where a wrong number matters most (`MILE-143`). Body prose outside the declared sections is deliberately not made checkable: a bare `#N` there is as often an example or a counter as a citation.

## Done means

- A number or repository link in a declared header field that is not in the committed snapshot is a finding naming the field and its line. — evidence: test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2, code:rust/crates/urzua-core/src/rules/citation.rs@sha256:a1c156d0395a2bcad18f5763b56e31b48d622a534b15cf0795df9b08cfd4758a
- With no `fields` declared the rule behaves as before. — evidence: test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2
- An undeclared or blank field name is a load error. — evidence: test:rust/crates/urzua-core/src/config.rs#issue_refs_fields_must_be_declared_header_fields_and_never_blank@sha256:3c1d9033c06d0894d26493fddc21ead263fc5f9fd3141161aae4dbb825327fb7, test:rust/crates/urzua-cli/tests/refs_integration.rs#a_declared_header_field_is_checked_end_to_end_and_only_when_declared@sha256:ca288cd98d9026d70c768cf8ea537ac934369039c667bdc01e546135092754e2
- This repository declares `fields: ["Found-in"]`, the only header field here that carries PR numbers, and `urzua check` is clean. — evidence: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Filed and delivered: `issue_refs.fields` (`SPEC-2`, `SPEC-3`), enabled for `Found-in` here; the snapshot was refreshed first and no dangling number was found. `Realized-by` and `Status: Done` set by the delivering change. **Why:** a wrong number in a header field passed unchecked; `MILE-143` was already Done, so this is a new milestone. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/rules/citation.rs@sha256:a1c156d0395a, .urzua/config.yaml@sha256:151830e5926d Reason: Pinned the files as they are now after checking each against the record it evidences: commands/fix.rs still re-pins on review with an attributed row (MILE-181), config.yaml, the Makefile and roadmap.rs still declare and run urzua roadmap (MILE-182), status_model.rs, rules/status.rs and config.yaml still compute a milestone's status from its delivery evidence (MILE-183), citation.rs and config.yaml still check the declared header fields for cited numbers (MILE-184), config.yaml still carries the narrowed scoped pins (MILE-185), the four condition records still exist and are Open as the milestone says (MILE-187), override_log.rs, .urzua/override-log.md, table.rs and generate-rule-table.py still hold the override-log rule (MILE-188), locator_mechanism.rs, table.rs, config.yaml and generate-rule-table.py still hold the lighter-mechanism warning (MILE-189), BUG-99 is still the Fixed record the audit-fold item cites (MILE-190), and check.rs, settings.json and urzua-claude-code.py still judge one record's proposed content under --stdin-path (MILE-191). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
