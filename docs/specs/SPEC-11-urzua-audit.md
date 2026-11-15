---
Stable-Id: 01M6R3NK5C6BXVCRM0Y2JS1DT5
Version: '0.17'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:7f54effa99c27c74c39869d05b39f69b13b2c030c24d2bd56db72fde59e86817, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:dc29b51bde5a2ba79d95c5e0a09466a54e816e8e2cc4a86a80e4459be477a447, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:919b855ba25bcabe5719a5edee34e9e5c3b9d8fe8f67ee7818f4a747971f13db
Author: beauwilliams
Subject: '`urzua audit` -- a deprecated alias of `urzua check` over the configured rules, with no rule set of its own.'
Implements: ADR-30
Parent: SPEC-1
---
# SPEC-11 — `urzua audit`

## Purpose

`urzua audit` was cross-record reconciliation kept apart from `check`: supersession reciprocity and
dangling cross-references. It ran two rule functions `check` already runs, over the same corpus,
producing the same report and exit codes (`RFC-38`). `ADR-53` made every rule a declared, opt-in
policy, so a rule set hardcoded inside the tool overrides the adopter's declaration; that ownership was
the root of `BUG-42`, `BUG-84` and `BUG-99`.

`ADR-79` folds it into `check`. Today `audit` is a **deprecated alias** and owns no rule set. It is
removed the release after.

## What it does

`urzua audit` runs `check`'s engine over the rules the repository's configuration declares and enables,
with no rule selection of its own, so its report **is** `check`'s: the same `status`, `findings`,
`rules_executed`, `rules_selected` (`null`), `notices` and exit codes (`SPEC-2`). It cannot disagree with
`check`, and it cannot exit 2 on a configuration `check` accepts. A corpus whose configuration declares
neither of the two rules `audit` used to run, such as the one `init` writes for a corpus with no type
prefix (`BUG-99`), is judged by whatever it does declare.

It takes no arguments beyond the global `--config`. A consumer migrating to `urzua check` changes the
invocation and nothing in the shape of what comes back; for the narrower report `audit` used to give,
`urzua check --rules pointer.resolution,relation.supersession-reciprocity` (a rule the configuration
does not declare is an error there, exit 2, rather than a silent empty run).

## The deprecation notice

Every report carries one `Notice` with `subject: "audit"` and severity `warning` naming the replacement,
`urzua check`. A `Notice` never moves the exit code (`ADR-46`), so no adopter's build breaks on the
warning. The only difference between an `audit` report and the `check` report is that notice.

## What's deliberately not built

- **Writing.** Permanently out of scope. The reciprocity checker must never write; repair is `fix`
  (`SPEC-8`), gated hard.
- **A rule set, a path scope or a flag of its own.** Anything `audit` could be asked for is a `check`
  invocation.
- **Removal.** The release after the deprecation release removes the command, with a clear message for a
  caller that still invokes it. Re-adding it afterwards would cost a deprecation cycle of its own
  (`ADR-79`).

## References

- ADR-79 — the decision: fold `audit` into `check`, and keep one read-only validator.
- RFC-38 — the proposal (sections 1 to 3 are built here).
- ADR-30 — the earlier decision that `audit` reuse `check`'s rule functions, which this supersedes in
  effect: it no longer calls them itself.
- ADR-53 — every rule is a declared policy.
- BUG-42, BUG-84, BUG-99 — the bugs rooted in `audit` owning a rule set.
- SPEC-2 — `check`, including `--rules` and `rules_selected`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial spec, giving `audit` its own complete build document rather than leaving it as an ADR describing only the *how*, with the *what* living separately in SPEC-1. **Why:** MILE-77 found this and `new`/`explain`/`graph`/`migrate` documented only as ADRs while comparable-complexity areas (`check`, `init`) had specs. | **structural** |
> | 2026-09-08 | Bumped to `0.2`. **Why:** MILE-74 decided `Author` is a required `spec` field, matching the accountability argument already applied to `adr`/`rfc` (MILE-78) -- backfilled with the real handle, not a placeholder. | **substantive** |
> | 2026-09-09 | Added the new required `Subject` field (`MILE-91`): a one-line summary of what this spec covers, readable without opening `Purpose`. | **structural** |
> | 2026-11-05 | Bumped to `0.4`. Added `Realized-by` naming the implementing code and the tests that exercise this spec: `audit_reports_a_one_directional_supersession_and_passes_a_reciprocated_one`. **Why:** the maintainer approved moving this spec to `Shipped`, which requires `Embodiment` and `Realized-by`; the tests run this repository's own `.urzua/config.yaml`. | **structural** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/commands/audit.rs, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Not started` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-05 | Status changed from Accepted to Shipped and bumped to `0.5`. **Why:** maintainer decision of 2026-11-05 to ship the specs whose implementation is verified; `Embodiment: Verified` with `Realized-by` naming the implementing code and tests. | **structural** |
> | 2026-11-05 | Bumped to `0.6`. Rewritten: `audit` is a deprecated alias of `check` over the configured rules with no rule set of its own, adding one deprecation `Notice`; removed the release after. **Why:** `MILE-190`, `ADR-79`. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/commands/audit.rs@sha256:80228f66e936, rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa23975490, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Bumped to `0.7`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 7caa23975490 -> 44fea1626a54. Reason: Read the diff of each pinned file since the old pin and the claim still holds. record_types_own_config_integration.rs: the milestone fixture's Done means is now a list item instead of a prose sentence (the repository config checks Done means as items), and the helper that swaps in items replaces that item; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.8`. The pin this spec's `Realized-by` holds on `record_types_own_config_integration.rs` was re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. **Why:** a revision-log entry that adds a structural change needs a `Version` bump (`revision-log.version-bump-required`). | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.9`. The `Realized-by` pin on `check_integration.rs` was re-baselined (the `Re-baselined` row above) after its tests were changed to name real tracked files; no behaviour this spec describes changes. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/commands/audit.rs` 80228f66e936 -> 7f54effa99c2. Reason: Read the diff of each re-pinned file against main. cli/src/commands/audit.rs: Notice struct literals gained detail: None or use Notice::new; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Bumped to `0.11`. The pins this spec's `Realized-by` holds on files this change touched were re-baselined (the `Re-baselined` row above); no behaviour this spec describes changed. | **structural** |
> | 2026-11-09 | Bumped to `0.12`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 5ab61b95fd58 -> a60c9c61b94c. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/tests/check_integration.rs: adds one regression test that a claim file with an upper-case .MD extension under claim_paths is read; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.13`. Evidence pins re-baselined for this change; the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/record_types_own_config_integration.rs` 44fea1626a54 -> dc29b51bde5a. Reason: Read the diff of each pinned test file since the previous pin. The repo() fixture helper now also copies the repository's .urzua/templates into the fixture, because the own config declares sections.order from templates; no assertion or test body changed; the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.14`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` a60c9c61b94c -> bc2e870defc6. Reason: Read the diff of each re-pinned file against main and the claim still holds. check_integration.rs: the ADR-0001 waiver fixture's H1 gains the citation-fixture marker text and every_header_reading_rule_reports_a_header_none_type_as_out_of_scope declares the new rule's required options; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Bumped to `0.15`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` bc2e870defc6 -> 3441046fd344. Reason: check_integration.rs changed by test hardening only: the three tests that need chmod to deny access now start with common::permission_test_can_run, so they skip loudly as root and fail in CI instead of passing silently, and 3 not-exit-0 assertions now pin the exact exit code; no assertion of the recorded behaviour was weakened **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.16`. The test files this spec's evidence pins were hardened (exact exit codes, isolated git, visible root skips); the pins are re-baselined in the row above and the specified behaviour is unchanged. **Why:** the review's test findings; a re-baseline row is structural, so the version moves with it. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 3441046fd344 -> 919b855ba25b. Reason: Read the diff of each pinned file against main and the claim still holds. check_integration.rs: the init-then-check blockquote test also enables header.required-fields after declaring required_fields, because adopt now withholds that rule; other tests unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Bumped to `0.17`. Evidence pins re-baselined for this change (the `Re-baselined` row above); the specification text is unchanged. **Why:** a revision-log entry that adds a structural change bumps `Version`. | **structural** |
