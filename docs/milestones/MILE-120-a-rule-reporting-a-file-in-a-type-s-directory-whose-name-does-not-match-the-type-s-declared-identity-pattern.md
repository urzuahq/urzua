---
Stable-Id: 01M6CXZVVVP6PK91GPCXN3SWSK
Status: Done
Phase: '0'
Track: schema-governance
Realized-by: code:rust/crates/urzua-core/src/rules/filename.rs#filename_identity_pattern@sha256:295f9d619beec55c4efaece2d8a21d2cd94f44080ea27158e74d66f98b2f093f, code:rust/crates/urzua-core/src/rules/filename.rs#identity_findings_for_type@sha256:5e99774ea061e90821752fe1a832c445467179ca99ac2dea570fe70223bbf735, test:rust/crates/urzua-cli/tests/check_integration.rs#the_filename_pattern_rule_is_opt_in@sha256:b29b3f4a1da8a4046d4d838d4d491c95e00317c6b2a3dac3efe2242e6362ebb6, test:rust/crates/urzua-cli/tests/check_integration.rs#one_off_pattern_file_among_conforming_ones_is_reported_on_that_file@sha256:954f4a745c9f79317d80d05ef9dd3fb4f30bd36a9ed26247020c55b5e9083179, test:rust/crates/urzua-core/src/rules/filename.rs#a_declared_pattern_decides_what_is_off_pattern@sha256:700062d5b4c3a4613c6d0757d8ce516f86f7a25b62f175d76abee88884606d26, test:rust/crates/urzua-cli/tests/check_integration.rs#a_bare_number_corpus_is_reported_once_per_type_with_rename_guidance@sha256:cbe75ec7f8dd363b5945a74a54669bd1ee721798e2be92e93273d8538909c31d, test:rust/crates/urzua-core/src/rules/filename.rs#a_whole_bare_number_corpus_is_one_finding_per_type_with_every_expected_name@sha256:0b7e4f1a7522745411866f1c66037f7b3ed24491f63203905e70def6529b533b, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5, test:rust/crates/urzua-cli/tests/check_integration.rs#a_large_bare_number_corpus_names_a_few_renames_and_counts_the_rest@sha256:554f574c983491ea05e84de4fed7701639ad1bc55c8bcc64e820bf552679528a
---
# 120 — A rule reporting a file in a type's directory whose name does not match the type's declared identity pattern

## What

A rule that reports a file in a record type's declared directory whose name does not match that type's
declared identity pattern. Today such a file is not a record of the type: it is silently absent from
every rule's population, and nothing says so.

## Why

A first real adopter corpus of several hundred records using bare zero-padded numeric filenames
(`0173-slug.md`) is exactly the case where a handful of files named off-pattern (a draft, a typo, a
renamed template) would vanish from every check while still sitting in the directory. `SPEC-1`'s
no-silent-no-op rule applies: a file the directory declares but the type cannot read is a finding, not
an absence. This is the file-level sibling of `type.record-outside-declared-dir`, which covers the
reverse direction. Needed for `MILE-102`.

Since `RFC-46` was rejected, renaming is the only supported route for a corpus in another filename shape, so this rule is also how the engine says so: when every file in a type's directory is off-pattern (a whole bare-number corpus), `urzua init` and `urzua audit` report the mismatch once per type, with an expected name for each file that conforms to the type's declared identity pattern (`TYPE-NNNN-slug.md` in this repository), instead of silently running the identity rules on zero records. That covers the bare-filename case of `BUG-99`, and the acceptance suite's bare-filename known gap becomes an assertion of this guidance.

## Done means

- The opt-in rule `filename.identity-pattern` reports a file in a type's declared directory whose name does not match the type's declared identity pattern, or the default `TYPE-NNNN-slug.md` when none is declared. — evidence: code:rust/crates/urzua-core/src/rules/filename.rs#filename_identity_pattern@sha256:295f9d619beec55c4efaece2d8a21d2cd94f44080ea27158e74d66f98b2f093f, code:rust/crates/urzua-core/src/rules/filename.rs#identity_findings_for_type@sha256:5e99774ea061e90821752fe1a832c445467179ca99ac2dea570fe70223bbf735, test:rust/crates/urzua-cli/tests/check_integration.rs#the_filename_pattern_rule_is_opt_in@sha256:b29b3f4a1da8a4046d4d838d4d491c95e00317c6b2a3dac3efe2242e6362ebb6, test:rust/crates/urzua-cli/tests/check_integration.rs#one_off_pattern_file_among_conforming_ones_is_reported_on_that_file@sha256:954f4a745c9f79317d80d05ef9dd3fb4f30bd36a9ed26247020c55b5e9083179, test:rust/crates/urzua-core/src/rules/filename.rs#a_declared_pattern_decides_what_is_off_pattern@sha256:700062d5b4c3a4613c6d0757d8ce516f86f7a25b62f175d76abee88884606d26
- A whole off-pattern type is one finding listing each file's expected name. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#a_bare_number_corpus_is_reported_once_per_type_with_rename_guidance@sha256:cbe75ec7f8dd363b5945a74a54669bd1ee721798e2be92e93273d8538909c31d, test:rust/crates/urzua-core/src/rules/filename.rs#a_whole_bare_number_corpus_is_one_finding_per_type_with_every_expected_name@sha256:0b7e4f1a7522745411866f1c66037f7b3ed24491f63203905e70def6529b533b
- The rule runs under `check` and under `audit`. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#the_filename_pattern_rule_is_opt_in@sha256:b29b3f4a1da8a4046d4d838d4d491c95e00317c6b2a3dac3efe2242e6362ebb6, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5
- A fixture covers an off-pattern, a pattern-matching and a legacy file. — evidence: test:rust/crates/urzua-cli/tests/check_integration.rs#one_off_pattern_file_among_conforming_ones_is_reported_on_that_file@sha256:954f4a745c9f79317d80d05ef9dd3fb4f30bd36a9ed26247020c55b5e9083179, test:rust/crates/urzua-cli/tests/check_integration.rs#a_large_bare_number_corpus_names_a_few_renames_and_counts_the_rest@sha256:554f574c983491ea05e84de4fed7701639ad1bc55c8bcc64e820bf552679528a

## References

- `type.record-outside-declared-dir` -- the existing reverse-direction rule.
- `MILE-98` -- the declared identity pattern this rule compares filenames against.
- `RFC-46` -- rejected; renaming replaces tolerating bare-number filenames.
- `MILE-134` -- the later rename tool.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** An adopter's off-pattern filename in a type's directory is invisible to every rule; filed as a first-adopter gap. | **substantive** |
> | 2026-11-02 | Scope widened to whole-corpus mismatch guidance in `init` and `audit`. **Why:** `RFC-46` was rejected, so the engine must explain the one convention rather than tolerate others. | **substantive** |
> | 2026-11-02 | Expected name follows the declared pattern. **Why:** A type may declare a pattern other than `TYPE-NNNN-slug.md`; files matching it are not off-pattern. | **substantive** |
> | 2026-11-04 | Built as the opt-in rule `filename.identity-pattern` (`SPEC-2`), run by `check` and `audit`; a whole off-pattern type is one finding on its directory listing each file's expected name, and the acceptance suite's bare-filename known gap is now an assertion of that guidance. `init` does not report it itself; it proposes the rule at `warn`, so the first `audit` does. **Why:** the milestone's rule and rename guidance are shipped; closing it is a human decision. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #166: opt-in rule `filename.identity-pattern` reports a file in a type directory whose name does not match the declared pattern. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/filename.rs#filename_identity_pattern` (lines 258-312) a80507e24452 -> 295f9d619bee. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: filename.rs: split filename_identity_pattern into identity_findings_for_type. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-core/src/rules/filename.rs#identity_findings_for_type@sha256:5e99774ea061 Reason: Pinned the helpers the evidenced logic moved to in the rules split, so the scoped evidence covers the logic and not only the entry function; behaviour unchanged **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
