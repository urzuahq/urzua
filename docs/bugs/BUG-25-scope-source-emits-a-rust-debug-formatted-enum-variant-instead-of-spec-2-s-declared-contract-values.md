---
Stable-Id: 01M28KB048PY0D3H4T5CGQXRPW
Status: Fixed
Found-in: 'reading `check`''s raw JSON while investigating BUG-24 -- `scope.source` came back as the literal string `"GitTracked"`, which is not one of the four values SPEC-2 declares'
Realized-by: code:rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb91c1658c5c90d4f08b4e8b3b6f193d408015508ad53888e9b37f, code:rust/crates/urzua-cli/src/discovery.rs@sha256:5f489efc9a95c213874196d2bb8fc5592fbecd07e4787cbe1a26f1ebbc6c7ab2, test:rust/crates/urzua-cli/tests/check_integration.rs#scope_source_is_the_declared_contract_value_not_a_debug_rendering@sha256:827fbffaf0cc13af3b1cf73b0d776f179fe58b7dc32fae0eab772d7d8f25000b
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
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs#scope_source_is_the_declared_contract_value_not_a_debug_rendering` (lines 43-65) c1d9b7b7f735 -> 827fbffaf0cc. Reason: Read the diff of each pinned span since the old pin and the claim still holds. check_integration.rs#scope_source_is_the_declared_contract_value_not_a_debug_rendering: the test body only dropped its manual remove_dir_all of the fixture directory (a TempDir guard removes it on drop), every fixture and assertion is unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-core/src/report.rs@sha256:60b6889bf4eb, rust/crates/urzua-cli/src/discovery.rs@sha256:eeef78dc2bb8 Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` eeef78dc2bb8 -> 5f489efc9a95. Reason: rust/crates/urzua-cli/src/discovery.rs changed only in compute_claim_measures, which now blames the first evidence line through the shared evidence_anchor_line instead of the Realized-by field line; the code this record cites is not touched by that change, so the claim stands (diffed, not re-read in full). **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
