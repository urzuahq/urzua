---
Stable-Id: 01M8677WVWXPFD5PX8Z0XKN998
Status: Fixed
Found-in: the review of the pull request that fixed BUG-230, on a record citing one file as spec and as record
Realized-by: code:rust/crates/urzua-cli/src/commands/fix_pin.rs#pinned_entry@sha256:8d73f53f7f87dbf1eda210d5e20b2076ef443b023f34d5cf32bbd5c21afe65b8, test:rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_refusal_names_only_the_tiers_a_file_is_cited_in_observed_failing@sha256:db704336dea6921b9165b27138302488d4fe8ba350954944e4bee90df7ec1591
---
# 231 — fix --pin names tiers the file is not cited in when a bare OLD matches two tiers

## What was wrong

A bare `OLD` for `urzua fix --pin --remove` or `--replace` that matches citations in two tiers is refused with "name the one to change as `code:{name}` or `test:{name}`" (`pinned_entry` in `rust/crates/urzua-cli/src/commands/fix_pin.rs`). The filter accepts every evidence category (`spec`, `code`, `test`, `record`), so on a file cited as `spec:` and `record:` the advice names two spellings that match nothing, and omits the two that work. `SPEC-8` listed only `code:` and `test:` as tier prefixes while `BUG-230` said any evidence category.

## Why nothing caught it

`BUG-230`'s test cited the file as `code:` and `test:`, the two tiers the message was written for, so the spellings it printed happened to be right.

## References

Fixed in `pinned_entry`: the spellings are built from the tiers actually found, in category order. `SPEC-8` ("Pinning evidence with a row") now says `OLD` may carry any evidence category. The tests are `fix_pin_refusal_names_only_the_tiers_a_file_is_cited_in_observed_failing` in `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs` and the acceptance case `fix-pin-names-the-tiers-a-bare-old-is-cited-in` in `corpus-g`, each observed failing before the fix.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-24 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 0 file-level, 2 scoped. Evidence: rust/crates/urzua-cli/src/commands/fix_pin.rs#pinned_entry@sha256:8d73f53f7f87, rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_refusal_names_only_the_tiers_a_file_is_cited_in_observed_failing@sha256:b75790054f9e Reason: BUG-231 pins pinned_entry and its refusal test; BUG-232 pins apply_remove and the field-copy test, the code and tests this PR changed. **Why:** a claim is renewed only over pinned code and test evidence. | **evidence** |
> | 2026-11-24 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/fix_rebaseline_integration.rs#fix_pin_refusal_names_only_the_tiers_a_file_is_cited_in_observed_failing` (lines 1949-1980) b75790054f9e -> db704336dea6. Reason: Read the diff of the test file since the first pin: cargo fmt reflowed the new tests' argument lists and one assert; their logic is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
