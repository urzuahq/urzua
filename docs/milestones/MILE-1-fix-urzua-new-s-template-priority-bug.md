---
Status: Done
Stable-Id: 01M1Y5JCCP2D9ES98WDB9V5PPF
Phase: '0'
Track: header-format
Implements: ADR-27, BUG-3
Blocked-on: —
Realized-by: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:1f7d3b1732152bd0af6a555d3f6d5f128115ce34e23d4e8943af5b455cc1d994, code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
---
# 1 — Fix urzua new's template-priority bug

## What

urzua new picks blockquote vs. YAML frontmatter by whether a template file exists, ignoring the configured header_shape entirely.

## Why

Flipping any type's header_shape to yaml-frontmatter without this fix means urzua new keeps emitting blockquote content that check can't parse under the declared shape -- silently broken output.

## Done means

- `urzua new` for a type configured `yaml-frontmatter` emits a YAML-frontmatter record whether or not a blockquote-shaped template file exists. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:1f7d3b1732152bd0af6a555d3f6d5f128115ce34e23d4e8943af5b455cc1d994, code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
- A test plants a blockquote-shaped template and fails if the output is blockquote (`new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present`). — evidence: test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
- The configured `header_shape` is the only thing that picks the output shape. — evidence: code:rust/crates/urzua-cli/src/commands/new.rs@sha256:1f7d3b1732152bd0af6a555d3f6d5f128115ce34e23d4e8943af5b455cc1d994

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: header_shape wins unconditionally over a template's own shape (BUG-3). | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/new.rs@sha256:5b61c497ff7e, rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5f Reason: Pinned the files as they are now after checking each against the record it evidences: claim.rs and claim_field.rs still read a changeset's declared closes field as a claim (BUG-66), release-invariants.test.sh still plants a release-prep version bump and an unmatched level and asserts the guards fail, and checks.yml now runs it on the unattended path (BUG-92, BUG-93, BUG-96), new.rs and new_record.rs still take the configured header shape over a template (MILE-1), filename.rs still compares the filename's type prefix and padded number with the H1 (MILE-10), config.yaml still declares config.scope-matches-nothing and checks.yml runs the gate (MILE-106, MILE-110), and header.rs, narrative.rs, pointer.rs, field.rs, fix.rs, the Makefile and checks.yml still read the declared relation field names with relation_field_literals.rs failing on a restated literal (MILE-107, MILE-109). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/new.rs` 5b61c497ff7e -> e4ca70265a45. Reason: Read the diff of each re-pinned file against main. cli/src/commands/new.rs: Notice struct literals gained detail: None; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/new.rs` e4ca70265a45 -> 1f7d3b173215. Reason: Read the diff of each re-pinned file against main. cli/src/commands/new.rs: Notice struct literals replaced by Notice::new (a constructor with detail: None), keeping the function inside the SPEC-22 ceiling; no behaviour change. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
