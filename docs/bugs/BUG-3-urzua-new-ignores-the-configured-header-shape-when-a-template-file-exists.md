---
Status: Fixed
Stable-Id: 01M1Y7W55XWYY7DJF2GF8PZX9T
Found-in: found live while writing ADR-33 -- flipped adr's header_shape to yaml-frontmatter and ran `urzua new adr`, expecting YAML frontmatter output, and got blockquote content back unchanged
Realized-by: code:rust/crates/urzua-cli/src/main.rs@sha256:17e9f09c5daf5669409e05d39e6a930d03ed5ce6ec39e92c77712210dcd15042, code:rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5ff34c2492d7ad9ca7c0a0a35fdc743df9fc76ae124a8210430e67, test:rust/crates/urzua-cli/tests/new_integration.rs@sha256:c319a5750aa11e1bd2b6be9fc6a2d707f34150c514d49c35290ad353f0ef61f7, test:rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present@sha256:abbf083803b8a4fe62e6e7b933ffdec9e9964180b452f0a00334eaca302cfbd0
---
# 3 — urzua new ignores the configured header_shape when a template file exists

## What was wrong

`render_from_template` in `new_record.rs` is chosen unconditionally whenever
`.urzua/templates/<type>.md` exists, regardless of what `header_shape` the type's config declares.
A type configured as `yaml-frontmatter` with an existing blockquote template still gets blockquote
output from `urzua new` -- silently inconsistent with what `check`/`fix` will then try to parse it
as.

## Why nothing caught it

No existing test constructs a type with both a template file present *and* a non-default
`header_shape` declared -- the two dimensions (template existence, configured shape) were never
tested in combination, only independently.

## How it was fixed

`run_new` now branches on `type_config.header_shape` first: a `yaml-frontmatter` type always calls
`render_synthetic_yaml`, regardless of whether a template file exists. A new `template_body`
function extracts everything from a template's first `## ` section heading onward, so a template's
scaffolding (`## What`, `## Why`, the revision log) still gets spliced in under the synthesized
header when a template happens to exist -- fixing the header shape doesn't cost the template's body.
Every other shape (the default, `blockquote`/`bold-list`) keeps using `render_from_template`
unchanged.

## References

- ADR-27 -- the original template-vs-synthesis decision this defect lives inside.
- ADR-33 -- where this was found live, and which explicitly named this as a required fix before
  this repo's own config could safely flip to yaml-frontmatter.
- MILE-1 -- the planned work that fixed it, cross-linked via this record's own filename
  reference and the milestone's `Implements` field.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: defect diagnosed, not yet fixed. | **structural** |
> | 2026-09-07 | Fixed: configured header_shape wins unconditionally over a template's own shape; template body still spliced in under synthesized frontmatter. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present` (lines 41-88) 4e3d1af68adb -> abbf083803b8. Reason: Read the diff of each pinned span since the old pin and the claim still holds. new_integration.rs#new_emits_yaml_frontmatter_when_configured_even_with_a_blockquote_template_present: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84, rust/crates/urzua-core/src/new_record.rs@sha256:21f5012c2b5f, rust/crates/urzua-cli/tests/new_integration.rs@sha256:c319a5750aa1 Reason: Pinned the files as they are now after checking each against the record it evidences: publish-release.yml still builds and uploads the archives itself (BUG-28), new_record.rs still renders only the configured YAML header and new_integration.rs still holds the test that a blockquote template does not override it (BUG-3), waiver.rs is_active still parses Expires as a date (BUG-31), prepare-release.yml still hands knope's nothing-to-release exit to release-guard.sh instead of failing (BUG-33), ci.yml's changeset job still reads the label live (BUG-34), knope.toml still releases from changesets only (BUG-35), main.rs and check_integration.rs still emit doctor as JSON (BUG-4), release-guard.test.sh still plants an unmatched-package fragment and asserts a nonzero exit (BUG-48), field_state.rs still carries the retired placeholder tokens (BUG-5), and generate-rule-table.py and the Makefile rule-table-check target still compare SPEC-2's rule table with the shipped rule set (BUG-52). BUG-29 and BUG-30 are not in this batch because the filename.title-consistency code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 9fae4c3edf84 -> f317c199a33d. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: only the doc comment on the Init command changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no flag, argument or behaviour changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` f317c199a33d -> 32196dc86f62. Reason: Read the diff of each re-pinned file. main.rs: added the global --as-of option on Cli, the fix_evaluation_date gate that refuses it with --apply and on commands that compare no dates, and the as_of() report helper; command dispatch otherwise unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 32196dc86f62 -> e3b1b6050ff1. Reason: Read the diff of each re-pinned file. main.rs: added the global --as-of option on Cli, the fix_evaluation_date gate that refuses it with --apply and on commands that compare no dates, the as_of() report helper, and parse_cli, which now wraps the unchanged argument parse and that gate; command dispatch otherwise unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-cli/src/main.rs` e3b1b6050ff1 -> 17e9f09c5daf. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: main.rs: split main into install_panic_hook, panic_if_requested, dispatch_migrate, dispatch_harness and dispatch_export to stay under the SPEC-22 function bound. Behaviour unchanged, outputs byte-identical (check, check --base, fix, fix --rebaseline, reverify and reverify --as-of JSON, stdout, stderr and exit compared against a binary built from origin/main). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
