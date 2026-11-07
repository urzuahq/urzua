---
Stable-Id: 01M6MP8T0T4AAJDC15G7PBQ49Q
Status: InProgress
Phase: '0'
Track: governance-process
Implements: ADR-79
Blocked-on: —
Realized-by: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#rules_restricts_the_run_names_the_filter_and_omits_the_rest@sha256:fb108d1ffdc50829aa364356227b44519022ccdcfa6761875bed9f5b78a0046a, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#status_and_the_exit_code_are_computed_over_the_selected_set@sha256:7400dd4a27cfdd58cfe64144ba7a3c0203e54adc55e8e988139608c2bfa5f65e, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#a_rule_the_config_does_not_declare_is_an_invocation_error@sha256:db5763c227a484d9cf888e4deb4e004da8d388420bb319f427afd7e3f8ca4aa5, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#an_unfiltered_run_reports_rules_selected_as_null@sha256:629292c53d805266dc97399289df80fba4b88989c7168df93fc36bca0c072cf1, spec:docs/specs/SPEC-2-urzua-check.md, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_on_a_clean_corpus_still_exits_0_with_the_notice@sha256:0bf63b7f44328b9520c3e5110fc8aca165782519536a21f25e89fec69e023aef, test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_on_the_config_init_writes_for_a_prefixless_corpus_matches_check@sha256:35da1961a8016dd9d92e1c87d3430e00f6c8fe907a7873819960b653d787ee39, code:docs/bugs/BUG-99-urzua-audit-exits-2-on-the-config-urzua-init-writes-for-a-corpus-with-no-type-prefix.md, spec:docs/specs/SPEC-1-cli.md, spec:docs/specs/SPEC-11-urzua-audit.md
---
# 190 — Fold audit into check: check --rules, rules_selected in the report, and audit as a deprecated alias of check

## What

`urzua check --rules <id>[,<id>...]` restricts a run to the named declared rules and the report gains a top-level `rules_selected`; an excluded rule is omitted from `rules_executed`, and a requested rule the config does not declare is an invocation error with exit 2. `urzua audit` becomes, now, a deprecated alias of `check` over the configured rules with no rule set of its own, emitting a `Notice` that names the replacement, and its report equals `check`'s. `BUG-99` closes with a test.

## Why

`ADR-79` (amended while Proposed) folds `audit` into `check`: `audit` owns a hardcoded rule set, which `ADR-53` made wrong, and that ownership is the root of `BUG-42`, `BUG-84` and `BUG-99`. `ADR-79` says the removal comes the release after; this milestone builds the fold and the deprecation, not the removal.

## Done means

- `check --rules a,b` runs only the named declared rules, omits the rest from `rules_executed` (never as `not-enabled`), and names the filter in `rules_selected`. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#rules_restricts_the_run_names_the_filter_and_omits_the_rest@sha256:fb108d1ffdc50829aa364356227b44519022ccdcfa6761875bed9f5b78a0046a
- `check --rules` computes `status` and the exit code over the selected set. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#status_and_the_exit_code_are_computed_over_the_selected_set@sha256:7400dd4a27cfdd58cfe64144ba7a3c0203e54adc55e8e988139608c2bfa5f65e
- An undeclared rule id given to `check --rules` is a could-not-run report (exit 2) naming it. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#a_rule_the_config_does_not_declare_is_an_invocation_error@sha256:db5763c227a484d9cf888e4deb4e004da8d388420bb319f427afd7e3f8ca4aa5
- `rules_selected` is absent or null on an unfiltered run as `SPEC-2` states. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#an_unfiltered_run_reports_rules_selected_as_null@sha256:629292c53d805266dc97399289df80fba4b88989c7168df93fc36bca0c072cf1, spec:docs/specs/SPEC-2-urzua-check.md
- `urzua audit` runs through `check`'s engine over the configured rules with no selection of its own, and its report equals `check`'s. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_is_check_over_the_configured_rules_plus_one_deprecation_notice@sha256:e190a2681bbc21edee5066f08c4a3ebc6e1d33fb3e089fa68b551839c37b4dc5
- `urzua audit` emits a `Notice` naming `urzua check` and never moves the exit code. — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_on_a_clean_corpus_still_exits_0_with_the_notice@sha256:0bf63b7f44328b9520c3e5110fc8aca165782519536a21f25e89fec69e023aef
- A test runs `audit` on the config `urzua init` writes for a prefixless corpus and shows it matches `check` (the `BUG-99` regression). — evidence: test:rust/crates/urzua-cli/tests/audit_fold_integration.rs#audit_on_the_config_init_writes_for_a_prefixless_corpus_matches_check@sha256:35da1961a8016dd9d92e1c87d3430e00f6c8fe907a7873819960b653d787ee39
- `BUG-99` is closed by that test. — evidence: code:docs/bugs/BUG-99-urzua-audit-exits-2-on-the-config-urzua-init-writes-for-a-corpus-with-no-type-prefix.md
- `SPEC-1`, `SPEC-2` and `SPEC-11` describe the new flag, the report field and the alias. — evidence: spec:docs/specs/SPEC-1-cli.md, spec:docs/specs/SPEC-2-urzua-check.md, spec:docs/specs/SPEC-11-urzua-audit.md
- `SPEC-8` describes the new flag, the report field and the alias. — waived: not-applicable -- SPEC-8 documents `urzua fix` and has never mentioned `check --rules`, `rules_selected` or `audit`; the delivering change (revision row of 2026-11-05) updated SPEC-1, SPEC-2 and SPEC-11, so naming SPEC-8 here was an error in the item
- This repository's own `urzua audit` and `urzua check --rules` are exercised against its corpus.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Filed. **Why:** `ADR-79` Accepted 2026-11-05 folds `audit` into `check`; the milestone builds the part that ships now (selection, the alias and the deprecation Notice), with removal the release after. | **substantive** |
> | 2026-11-05 | Built in #311: `check --rules`, `rules_selected`, `audit` as a deprecated alias of `check` with one `Notice` (`SPEC-1`, `SPEC-2`, `SPEC-11`), and `BUG-99` fixed with its regression test. The command is removed the release after (`ADR-79`), which this milestone does not do. `Realized-by` added and `Status` set to `Done`. **Why:** every item of Done means is met and exercised in this repository's corpus. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Status` from `Realized-by computes to Verified` -- stated `Planned` disagreed with the computed value `Done`. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-07 | `Status` `Done` -> `InProgress`. Status set from its Done-means items (ADR-96, MILE-215): unresolved item 11. **Why:** the item has no evidence clause, and `DoneWithUnresolvedItems` forbids `Done` while an item is unresolved; `fix` never downgrades, so the status is set by hand to the computed value. | **structural** |
