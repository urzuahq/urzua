---
Stable-Id: 01M26JPHSH390CDZV1VQVP45FG
Status: Fixed
Found-in: 'reviewing where resolve_identity''s gh-vs-by divergence warning should live -- asked whether there''s a single generic output function every command already goes through, and found there isn''t'
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:83a590ba0c3ea3377a5734245d93c29ef55ab836fb21ac6ccd9af89e9df19760, code:rust/crates/urzua-cli/src/main.rs@sha256:ae71a0957f580500aec00ae803d0ce7c7efb435a5d3a37bf981e60dbf8bedc13, test:rust/crates/urzua-core/src/report.rs#notices_never_move_a_successful_exit_code@sha256:ae2ae7844fb3ef6287744441eeb949a16ac874326e7ed75f817b67cfde4d8746, test:rust/crates/urzua-cli/tests/check_integration.rs#fix_could_not_run_emits_json@sha256:3c19eb5997529c40ff9d2f64f5f34bdf0f2db19dcc43ff5b97f670bd2c566b83, test:rust/crates/urzua-cli/tests/check_integration.rs#explain_could_not_run_emits_json@sha256:1d06a1d80aa3059a75170cf25f732a714eafef07169d382b1943031675546a62, test:rust/crates/urzua-cli/tests/check_integration.rs#graph_could_not_run_emits_json@sha256:7557a40f07ae8c953456434234ba8fc3a88b71e61c161b1e4c7a19c126885aa3, test:rust/crates/urzua-cli/tests/check_integration.rs#doctor_missing_config_emits_parseable_json_with_checks@sha256:4717c27fcf7152f55b17574cc62f27022b13eb9dd1b0a27c4a19cc2473d83248
---
# 19 — Five CLI commands hand-roll their own JSON shape instead of one shared contract

## What was wrong

ADR-7's own Consequences section states plainly: "every future command (`audit`, `migrate`,
`export`/`import`, `doctor`) must emit this same shape, not invent its own -- a second, incompatible
JSON shape would defeat the point." That never happened. Today there are three different, mutually
incompatible output paths:

- `check` -> `print_report(&CheckReport)`, the rich shape (`status`, `files_examined`,
  `rules_executed`, `scope`, `blocking`, `findings`).
- `fix` -> `print_fix_report(...)`, a second, different ad hoc `serde_json::json!({...})` block.
- `new`, `explain`, `graph`, `doctor` -- each builds its own one-off `serde_json::json!({...})`
  inline in its own `run_*` function, no shared function at all.

Compounding this: `Finding` (the type `check`'s rich report uses for anything worth surfacing) has a
mandatory `file: PathBuf` field, so it can't represent a notice that isn't about a specific file --
exactly the shape of `resolve_identity`'s gh-vs-`--by` divergence warning, which is about a flag
value, not a record. That warning was originally implemented as a bare `eprintln!` from inside
`urzua-io` (a library crate, with no access to any of the CLI's JSON machinery even if a shared shape
existed) -- unstructured stderr prose an agent has no reliable way to parse, in a tool whose own
entire pitch is agent-native structured output. It was reworked to return the warning as data
(`ResolvedIdentity { name, warning }`) and threaded into `new`'s and `fix`'s own existing per-command
JSON as an ad hoc `"warnings"` array -- a real fix for that one call site, but it papers over the
actual gap rather than closing it: the next command that needs to surface a non-blocking notice has
nothing to reuse and will invent a fourth shape.

A related, smaller instance of the same root problem: `rust/crates/urzua-cli/src/main.rs`'s
`run_fix`'s tier-not-implemented guard (`"only tier 1 is implemented so far (ADR-0015 defines tiers 2
and 3, not yet built)"`) and the identity-divergence warning both cited an internal ADR number
directly in user-facing output -- meaningless to anyone running this tool against their own corpus,
where no such ADR exists. The identity-divergence instance was fixed (the ADR reference dropped from
the message text); the tier-not-implemented one was not touched, since it's a pre-existing message on
a code path unrelated to this fix and re-plumbing it means also giving that early-exit guard clause
the same treatment described below.

## Why nothing caught it

`check`'s single, well-exercised output path made it easy to assume the "one shape" decision was
actually enforced everywhere, when it was only ever built where `check` needed it. Nothing greps for
`serde_json::json!(` call sites outside `report.rs` and flags a shape that doesn't reuse
`CheckReport`/`Finding` -- there's no rule or test asserting "every command's stdout parses as one of
N declared shapes." Each new command shipped independently, and each one's own `json!({...})` block
looked locally reasonable in isolation.

## References

- `rust/crates/urzua-core/src/report.rs` -- `CheckReport`, `Finding`, `RuleExecution`: the one shape
  that actually exists.
- `rust/crates/urzua-cli/src/main.rs` -- `print_report`, `print_fix_report`, and the inline
  `serde_json::json!({...})` blocks in `run_new`, `run_explain`, `run_graph`, `run_doctor`.
- ADR-7 -- "every future command ... must emit this same shape, not invent its own," the decision
  this bug reports as unrealized.
- ADR-23 -- "every entry point" emits JSON unconditionally; doesn't specify a shared shape beyond
  `check`/`fix`/`migrate schema --report`.
- `rust/crates/urzua-io/src/lib.rs` -- `resolve_identity`/`ResolvedIdentity`, the specific call site
  that surfaced this.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-10 | Initial bug record, `Status: Open`. Not yet fixed -- whether the right shape is one generic envelope every command adopts, a narrower shared `warnings: Vec<String>` convention, or something else is not yet decided. | **structural** |
> | 2026-09-11 | `Status: Fixed` (ADR-46). Neither candidate named above: a generic envelope was tried and killed by a confirmed duplicate-JSON-key bug (`#[serde(flatten)]`-composing a shared `status` field into `CheckReport`, which already owns one); `warnings: Vec<String>` was replaced, not kept, since a bare string can't carry `severity`/`subject`. Landed instead: a `Report` trait (`notices()`/`exit_code()`) plus a real `Notice` type, no shared struct composition. `check`/`fix`/`new`/`explain`/`graph`/`doctor` all migrated; `init`/`migrate ids` remain out of scope (plain-text prose end to end, filed separately). | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 5 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs#notices_never_move_a_successful_exit_code` (lines 1446-1471) f58623bec0db -> 6c73c0408a77, `rust/crates/urzua-cli/tests/check_integration.rs#fix_could_not_run_emits_json` (lines 701-715) 61ac67a58509 -> 3c19eb599752, `rust/crates/urzua-cli/tests/check_integration.rs#explain_could_not_run_emits_json` (lines 743-754) ca1589422ef2 -> 1d06a1d80aa3, `rust/crates/urzua-cli/tests/check_integration.rs#graph_could_not_run_emits_json` (lines 756-767) 4cade01073a0 -> 7557a40f07ae, `rust/crates/urzua-cli/tests/check_integration.rs#doctor_missing_config_emits_parseable_json_with_checks` (lines 1014-1028) edc92f45f48b -> 4717c27fcf71. Reason: Read the diff of each pinned span since the old pin and the claim still holds. report.rs#notices_never_move_a_successful_exit_code: the test's Config or report fixture literal gained newly added default fields (for example roadmap, attestation, new, discovery, harness, authority, permanent, rules_selected, include_untracked), assertions unchanged; check_integration.rs#fix_could_not_run_emits_json: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#explain_could_not_run_emits_json: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#graph_could_not_run_emits_json: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged; check_integration.rs#doctor_missing_config_emits_parseable_json_with_checks: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb, rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84 Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 9fae4c3edf84 -> f317c199a33d. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: only the doc comment on the Init command changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no flag, argument or behaviour changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 60b6889bf4eb -> f8332710ffd7, `rust/crates/urzua-cli/src/main.rs` f317c199a33d -> 32196dc86f62, `rust/crates/urzua-core/src/report.rs#notices_never_move_a_successful_exit_code` (lines 1467-1496) 6c73c0408a77 -> ae2ae7844fb3. Reason: Read the diff of each re-pinned file. main.rs: added the global --as-of option on Cli, the fix_evaluation_date gate that refuses it with --apply and on commands that compare no dates, and the as_of() report helper; command dispatch otherwise unchanged; report.rs: added the AsOf and AsOfSourceKind types and an as_of field on CheckReport and FixReport; the existing fields, notices and exit_code contract are unchanged; report.rs test: the CheckReport literals gained the as_of field; the assertions are unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 32196dc86f62 -> e3b1b6050ff1. Reason: Read the diff of each re-pinned file. main.rs: added the global --as-of option on Cli, the fix_evaluation_date gate that refuses it with --apply and on commands that compare no dates, the as_of() report helper, and parse_cli, which now wraps the unchanged argument parse and that gate; command dispatch otherwise unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` f8332710ffd7 -> 96587e15e363. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: report.rs gained an additive Population counter excused_lines (skipped from the JSON when zero) with an accessor and a with_excused_lines builder; nothing else in the report contract changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 96587e15e363 -> fd0b8c483216, `rust/crates/urzua-cli/src/main.rs` e3b1b6050ff1 -> c7e27293c39c. Reason: Read the diff of each re-pinned file against main. core/report.rs: Notice gains an optional detail map of untagged string or string-list values, with DetailValue and Notice::new; cli/main.rs: the command about text states the product positioning; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` c7e27293c39c -> 0b6a713c8273. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: main.rs: split main into install_panic_hook, panic_if_requested, dispatch_migrate, dispatch_harness and dispatch_export to stay under the SPEC-22 function bound; main's #445 --help about text is unchanged. Behaviour unchanged, outputs byte-identical (check, check --base origin/main, fix, fix --rebaseline, reverify, reverify --as-of 2027-06-01 and --help stdout, stderr and exit compared against a binary built from origin/main). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` fd0b8c483216 -> 83a590ba0c3e. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: report.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; adds Finding::new, at, at_opt, declared, declared_opt and RuleExecution::ran, not_applicable, not_enabled; no existing item changed. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 0b6a713c8273 -> ae71a0957f58. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: the Init command gained the --types and --dir options and now passes them to init::run as an InitArgs, with their help text; the code this record cites (its own command's behaviour) is untouched. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
