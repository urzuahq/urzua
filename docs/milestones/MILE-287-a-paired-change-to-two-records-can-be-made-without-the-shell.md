---
Stable-Id: 01M7HGMM603206VS99JM6P9PRZ
Status: Done
Phase: '1'
Track: accountability-identity
Blocked-on: null
---
# 287 — A paired change to two records can be made without the shell

## What

An agent can make a change that needs two records to agree, such as one record superseding another under a declared relation pair, through the edit tools alone. `urzua fix` computes and writes the missing other half of a declared relation, the reciprocity findings name that command as their suggested action, and the editor hook lets the first half through by default while telling the agent exactly which record, field and value are still required.

## Why

Reported by the maintainer on 2026-11-16 from an agent working in a repository that adopted urzua: "The editor hook refused each half on its own, because the pair doesn't match until both sides exist. So I wrote all four lines in one shell step rather than taking a hook override, which the harness reserves for the maintainer." The editor hook judges one file's proposed content, so `relation.pair-reciprocity` and `relation.supersession-reciprocity` refused the first half because the other record did not point back yet, and the second half for the mirror reason. The only way left was the shell, which the harness cannot stop and only reports afterwards: the harness caused the outcome it exists to prevent. urzua's users are coding agents, so a refusal must come with a way forward that is not the shell.

## Done means

- `urzua fix` lists the missing inverse of a declared relation as a repair and, with `--apply --ids`, writes it on the other record with a tool-authored revision-log row; it never changes a `Status`, never overwrites a different existing value, appends to a list-valued field, writes nothing for a target that does not exist or a type the pair is not declared for, and works in both directions. — evidence: test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#fix_lists_inverses_for_a_list_valued_claim_and_appends_to_an_existing_list@sha256:cf0013aaada67dcaa51948b26f737020e88ca42768db29a812cd6d55a0d92ca6, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#fix_writes_the_forward_side_when_only_the_inverse_was_written@sha256:d51760b2c63714979755405842f4e6628583f68ceb6b01ddae10f6cfa3150f69, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#fix_never_overwrites_a_different_value_and_reports_the_conflict@sha256:a6a70210b4934989026ab7ff0c8115c31796b1d220f0428637f993505f1262a8, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#fix_writes_nothing_for_a_target_that_does_not_exist_and_says_so@sha256:c1cc75241229bded060e32c228960392d2627549527c42e2933fe897ea932bb8, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#fix_writes_nothing_to_a_type_the_pair_is_not_declared_for@sha256:87a9003ce730fc761af077ed88c962b273d1082c0f57a4bb5eeb2ed6fde6c2bf
- With `harness.guardrails.editor-hook.pending_pair: refuse`, the refusal of either half names the `urzua fix` command, and running it as printed after writing the first half makes `urzua check` clean. — evidence: test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#refuse_blocks_each_half_and_names_the_fix_command_that_makes_check_clean@sha256:3d68b12a1918153b5dadbda6a9704a4803330c4587baab285eaee4a205817854
- By default (`pending_pair: allow`), the first half is allowed with the record, field, value and `urzua fix` command still required, the second half is then allowed, an unrelated error in the same edit still refuses, a gated status move is still refused, and an unknown value stops the configuration loading. — evidence: test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#by_default_the_first_half_is_allowed_with_the_follow_up_and_the_second_completes_it@sha256:63ddf1336a29115afe239165f1f6d47ee5ae401bf6eecc7be07e4f0405fd91e6, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#an_unrelated_error_in_the_same_edit_still_refuses_and_names_the_pair_follow_up@sha256:cd15cb1c954fce814e3964f6d2acc6c7df1a65e5f13f1143cf5efa72d8d3a5b7, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#a_gated_status_move_with_the_second_half_is_still_refused@sha256:3a5e6ce14c35372c697030426f9dc389425792e6823ba9084abe705eff9379f7, test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#an_unknown_pending_pair_value_stops_the_load@sha256:766c51e2fb881bb01782ddc1abac96ecf2d0e90a87e19d1fa5ade154a38b1f4c
- An unmatched pair left in the working tree still fails `urzua check`, which the post-command hook, pre-commit and CI run. — evidence: test:rust/crates/urzua-cli/tests/pending_pair_integration.rs#an_unmatched_half_left_in_the_tree_still_fails_check@sha256:080598cf6e34897677d4b6e673acc41e023486a7fcf01fbaa23d817d20505128

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record. **Why:** the editor hook refused each half of a reciprocal relation, so an agent fell back to the shell; this milestone files the `urzua fix` inverse repair, the refusal that names it, and the `pending_pair` key (default `allow` by the maintainer's decision of 2026-11-16) delivered in the same pull request. | **structural** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (4 of 4 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
