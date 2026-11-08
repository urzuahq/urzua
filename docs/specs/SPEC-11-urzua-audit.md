---
Stable-Id: 01M6R3NK5C6BXVCRM0Y2JS1DT5
Version: '0.6'
Date: 2026-09-07
Status: Shipped
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/commands/audit.rs@sha256:80228f66e936a465168f1f2a4e8cbca8070fb1dfd42a28c087af2eeaec6d7a5e, test:rust/crates/urzua-cli/tests/record_types_own_config_integration.rs@sha256:7caa2397549099e9e22e83bed990459dc5318b64a2dea6d5770e9b3929301c1e, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b8337c1172d3c4825859e00a0344dd4c35e6f013d802e509b1688ecb
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
