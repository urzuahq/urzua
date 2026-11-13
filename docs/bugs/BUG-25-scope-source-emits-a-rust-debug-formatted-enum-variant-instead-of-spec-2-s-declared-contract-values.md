---
Stable-Id: 01M28KB048PY0D3H4T5CGQXRPW
Status: Fixed
Found-in: 'reading `check`''s raw JSON while investigating BUG-24 -- `scope.source` came back as the literal string `"GitTracked"`, which is not one of the four values SPEC-2 declares'
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:e4387ad0f746f9d6e23f403c03d2488d12a5993bf0bf80dbfa69b5ff612c3a82, code:rust/crates/urzua-cli/src/discovery.rs@sha256:ab2ed10a227b2d01506cf4ef3d9e7149f87566b2261fe4f213d97dfd93080968, test:rust/crates/urzua-cli/tests/check_integration.rs#scope_source_is_the_declared_contract_value_not_a_debug_rendering@sha256:827fbffaf0cc13af3b1cf73b0d776f179fe58b7dc32fae0eab772d7d8f25000b
---
# 25 — scope.source emits a Rust Debug-formatted enum variant instead of SPEC-2's declared contract values

## What was wrong

SPEC-2 declares four values for `scope.source`: `tracked-sweep`, `git-diff`, `argv`, `none`, in a
table whose whole purpose is that a consumer does not have to key "did this run" on a missing field.
Its worked example prints `"scope": { "source": "tracked-sweep", "base": null }`.

What `check` actually emits is `"source": "GitTracked"`, produced by
`source: format!("{:?}", discovered.source)` -- the `Debug` rendering of a `urzua-io` enum variant,
serialized straight into the public JSON contract. No declared value is ever emitted by a real run.
`base` is absent from the struct entirely, and an undeclared `record_types` array is present
instead.

The contract is the product here (ADR-23, RFC-3): the reason stdout is always JSON is that an agent
branches on these fields without interpreting anything first. An agent written against SPEC-2 that
branches on `scope.source == "tracked-sweep"` takes the wrong branch on every invocation. Worse,
the value is a `Debug` rendering of an internal type, so it is not merely undeclared but *unstable*:
renaming the `DiscoverySource::GitTracked` variant -- an ordinary internal refactor with no
contract intent -- silently changes public output. `report_could_not_run` separately emits
`"unavailable"`, a fifth undeclared value, from a hand-written string.

## Why nothing caught it

No test asserts the *value* of `scope.source` against SPEC-2's declared set; the field's presence
is what gets exercised. `ScopeInfo.source` is typed `String` rather than a serde-tagged enum, so
there is no compile-time relationship between the contract and what is emitted -- the `format!`
call is invisible to both the type system and the test suite. `DiscoverySource` having a single
variant also means the mismatch never varies from run to run: output that is uniformly wrong reads
as consistent, and consistent output is what a reviewer skims past.

Same class as BUG-4 (`doctor` emitted plain text where the contract said JSON) and BUG-6 (`render`
wrote unescaped YAML): the shape of the output was never asserted against the document that
specifies it.

## Two corrections to this record

**The `"unavailable"` fifth value is gone.** This record noted `report_could_not_run` emitting it from
a hand-written string. `ADR-0046` replaced that helper with `CouldNotRun`, which carries no scope at
all, so the value no longer exists.

**The leak was in two command modules, not `main.rs`.** `ADR-0046`'s split moved it to
`commands/check.rs:122` and `commands/audit.rs:73`. This record predates that.

## The fix

`ScopeSource` is a serde-renamed enum in `urzua-core::report`, beside `ReportStatus` and
`FindingSeverity`, and `ScopeInfo.source` now holds it. The value is a declared contract value, so a
rename of the internal `DiscoverySource` can no longer change public output.

It lives in `urzua-core` rather than `urzua-io` because of the boundary `ADR-0005` enforces: the two
crates cannot see each other, and `urzua-cli` is the only crate that depends on both. The adapter is
therefore a single `discovery::scope_source()` — one function rather than one per command, so `check`
and `audit` cannot disagree about the same fact (`ADR-0030`; the same defect `BUG-0011` found when
`graph` built its own index beside `pointer_resolution`'s).

`SPEC-0002` declares four values; only `tracked-sweep` is producible today, and the table now says so
rather than documenting three modes that do not exist.

## References

- `rust/crates/urzua-core/src/report.rs` -- `ScopeInfo`, whose `source: String` is the type-level
  gap; a serde-renamed enum would close it.
- `rust/crates/urzua-cli/src/main.rs` -- the two `format!("{:?}", discovered.source)` sites and the
  `"unavailable"` literal in `report_could_not_run`.
- `rust/crates/urzua-io/src/lib.rs` -- `DiscoverySource`, the internal type currently leaking into
  public output.
- SPEC-2 -- the `scope.source` table and the worked example this output contradicts, including the
  `base` field that is not implemented.
- ADR-23, RFC-3 -- always-JSON stdout and the agent-native contract this undermines.
- BUG-24 -- filed from the same run; its reproduction shows this value.
- BUG-4, BUG-6 -- same class: output shape never asserted against its own spec.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Fixed, and two of this record's own claims corrected. **Why:** the value is the first thing an agent branches on, and it matched no declared value on any run since the field existed. Chosen as a deliberately small user-facing change to exercise the changeset and release path end to end after a day of pipeline repairs. | **substantive** |
> | 2026-09-11 | Initial bug record, `Status: Open`. Not fixed -- the fix touches the published JSON contract (a serde-tagged enum, the missing `base`, the undeclared `record_types`), so it needs a decision and a changeset, not a silent rename. | **structural** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#scope_source_is_the_declared_contract_value_not_a_debug_rendering` (lines 43-65) c1d9b7b7f735 -> 827fbffaf0cc. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#scope_source_is_the_declared_contract_value_not_a_debug_rendering: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb, rust/crates/urzua-cli/src/discovery.rs@sha256:eeef78dc2bb8 Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` eeef78dc2bb8 -> 5f489efc9a95. Reason: rust/crates/urzua-cli/src/discovery.rs changed only in compute_claim_measures, which now blames the first evidence line through the shared evidence_anchor_line instead of the Realized-by field line; the code this record cites is not touched by that change, so the claim stands (diffed, not re-read in full). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 60b6889bf4eb -> f8332710ffd7. Reason: Read the diff of each re-pinned file. report.rs: added the AsOf and AsOfSourceKind types and an as_of field on CheckReport and FixReport; the existing fields, notices and exit_code contract are unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 5f489efc9a95 -> a0044b046d01. Reason: discovery.rs split compute_drifted_records_at into DriftJudge and its helpers, read_claim_files into claim_markdown_paths, and shared one resolved_model; blames go through a per-run BlameCache; behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` f8332710ffd7 -> 96587e15e363. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: report.rs gained an additive Population counter excused_lines (skipped from the JSON when zero) with an accessor and a with_excused_lines builder; nothing else in the report contract changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 96587e15e363 -> fd0b8c483216, `rust/crates/urzua-cli/src/discovery.rs` a0044b046d01 -> 7f85dbcd6026. Reason: Read the diff of each re-pinned file against main. core/report.rs: Notice gains an optional detail map of untagged string or string-list values, with DetailValue and Notice::new; cli/src/discovery.rs: Notice literals gain detail: None; no behaviour change. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 7f85dbcd6026 -> 8ea59ee343dc. Reason: Read the diff of each pinned file since the old pin and the claim still holds. discovery.rs: read_pull_ref_claims' git closure is urzua_io::git::stdout_in (same 'could not run git' / 'git <cmd> failed' wording); nothing else changed; behaviour unchanged, outputs byte-identical. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` fd0b8c483216 -> 83a590ba0c3e. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: report.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged; adds Finding::new, at, at_opt, declared, declared_opt and RuleExecution::ran, not_applicable, not_enabled; no existing item changed. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 8ea59ee343dc -> 8c83e667b37b. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. rust/crates/urzua-cli/src/discovery.rs: the drift computation judges every locator of a claim instead of stopping at the first drift: DriftedRecords gains a per-record `locators` map, Judged gains a Fresh variant, a new `note` helper records each verdict with its category, and the judge_* helpers return a verdict instead of inserting into the sets; which records are drifted or fresh is unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 8c83e667b37b -> ba6412881ce2. Reason: Read the diff of each pinned file against main. discovery.rs: a held-back drift now carries the time it leaves the grace window and the drift result records it for the notice; which locators drift and which are held back are unchanged; so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` ba6412881ce2 -> e9bea8f7e9a4. Reason: discovery.rs: drift-grace notice wording states the window end in UTC; LocatorDrift no longer carries inside_grace; **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` e9bea8f7e9a4 -> 875922a6e902. Reason: Read the diff of each pinned file against main. rust/crates/urzua-cli/src/discovery.rs: DriftJudge reads the record's Re-verified rows in the tool's exact shape that acknowledged drift and, for a category whose drift policy is acknowledge, treats a change of an unpinned locator that a row names, on or before that row's day, as acknowledged (locator_name, acknowledged_through, acknowledged); compute_base_drift is new and judges the merge base's copy of a record for check --base; note records each drifted locator's name; no other judgement changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 875922a6e902 -> 0a0f6e76f933. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/src/discovery.rs: the acknowledged-row reader skips a Re-verified row dated after the evaluation date (today or --as-of), so a future-dated row clears no drift; no other judgement changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 0a0f6e76f933 -> 46fb265a9f06. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/src/discovery.rs: the claim_paths walk matches the md extension case-insensitively; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-10 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 46fb265a9f06 -> ab2ed10a227b. Reason: discovery.rs: resolved_model made pub(crate), no behaviour change; the claim holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/report.rs` 83a590ba0c3e -> e4387ad0f746. Reason: Read the diff of each re-pinned file against main. report.rs: added WaiveReport, WaiveDraft and WaiveFindings with their Report impl for urzua waive; no existing type or function changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
