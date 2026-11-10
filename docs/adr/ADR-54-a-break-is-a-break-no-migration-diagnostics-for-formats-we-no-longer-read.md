---
Stable-Id: 01M2VHJHA4F5PMMA9DGW0CM6Y5
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/discovery.rs@sha256:46fb265a9f064396942b74bf89f27671e48a8866253032979d8c08e452687de0, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:a60c9c61b94c5707e0aa4e96713a843556cd50bce97e01c514826d61ca0aac08
Date: 2026-09-19
Accepted-on: 2026-09-19
Author: beauwilliams
Deciders: beauwilliams
Derives-from: ADR-52
Supersedes / Superseded-by: —
---
# 54 — A break is a break: no migration diagnostics for formats we no longer read

## Context

`ADR-52` moved the config from TOML to YAML as a hard break. Nothing parses TOML; `toml` is not a
dependency.

A pre-release review then found that `init` and `doctor` did not *notice* a leftover `config.toml`
(`BUG-47`). `doctor` advised running `init`, and `init` wrote a freshly auto-detected config beside
the existing one, leaving its `required_fields`, `known_fields`, `pointer_fields` and `spec` pointers
ignored, and exited 0. The first fix added a four-line detector shared by three commands.

It reads nothing and parses nothing. It exists solely to produce a better message. The question it
raises is whether that is worth carrying, and the answer turns on how many repositories it serves.

**Measured, 2026-09-19:**

| release | published | asset downloads |
|---|---|---|
| v0.3.0 | 2026-09-16 | **1** |
| v0.2.1 | 2026-09-16 | 4 |
| v0.1.0 | 2026-09-05 | 10 |

The population holding a TOML config is this repository, whose config was converted by hand three days
ago. The diagnostic serves nobody.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| **Remove it now** | No code encoding a dead format; nothing to find and be afraid of later | A future adopter on v0.3.x gets a less helpful message |
| Keep it, time-boxed to a release | Helps the hypothetical upgrader; has an end | The end is a date nobody will act on, and it is four lines of dead weight until then |
| Keep it indefinitely | Maximally helpful | Every format change accretes one, and none is ever removed |

## Decision

In the context of a format break whose migration population is measurably empty, facing four lines
that exist only to describe a format the tool no longer reads, **we decided to remove the diagnostic
and to treat this as the general rule: a break ships no migration aid**, to keep the codebase free of
knowledge about formats it does not support, accepting that an adopter upgrading across such a break
gets a plain "file not found" rather than an explanation.

The general rule matters more than this instance. `MILE-98` will change how fields, identity and
sections are declared, and `RFC-33` will change what a rule looks like. Each is a candidate for its
own compatibility shim, and *"we kept the last one"* is the argument that makes the second one
inevitable.

## Reversibility

Trivial, and that is part of the argument. The diagnostic is four lines and one message; if a real
adopter ever hits it, re-adding it costs an afternoon and arrives with evidence instead of a guess.

Removing it is cheap to undo. Keeping it is what compounds.

## Consequences

- **`BUG-47`'s defect is not reintroduced by this, but its fix is.** Nothing is deleted from disk
  either way: a leftover `config.toml` is ignored, not destroyed. What is lost is the *notice*, and
  the notice served a population of zero.
- **A future break must decide this again, and now has a default.** Not a precedent to follow blindly:
  a break with real adopters behind it is a different question, and this record exists so that
  question is asked with numbers rather than instinct.
- **`init` writing a config beside a leftover one is now ordinary behaviour.** A reader of `init.rs`
  will not find a guard explaining why; this record is the explanation.
- **The version gate still exists.** `schema_version` remains the mechanism that makes a break loud
  (`ADR-12`), and this decision does not touch it. A config from a *supported* schema version that
  cannot be read is still an error that names the version.
- **Note (2026-11-05): "a population of zero" holds through Phase 0 only.** It expires at the Phase 1 switchover
  (`MILE-102`), when a real adopter's config exists. From then on a break is the question this record says to ask
  "with numbers rather than instinct", and `ADR-77` (proposed) amends the default: a config schema break ships a
  support window and a migration path (`RFC-49`). The Decision above is unchanged for any format with no real
  adopter behind it (`MILE-118`).

## References

- ADR-52 -- the break this follows from.
- ADR-12 -- `schema_version`, the mechanism that stays.
- BUG-47 -- the defect, and the fix this removes before it ships.
- MILE-98, RFC-33 -- the next two breaks, and why the general rule is worth settling now.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Initial decision, `Status: Accepted`. **Why:** a review fix added a legacy-config detector, and the question of whether to carry it turned on how many repositories it serves. v0.3.0 has one asset download three days after release, so the population is this repository and its config was converted by hand. Decided as a general rule rather than a one-off, because `MILE-98` and `RFC-33` are each a candidate for the same shim and "we kept the last one" is what makes the second inevitable. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/discovery.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit fa5476f14. | **structural** |
> | 2026-11-05 | Dated note added to Consequences: the zero-population premise holds through Phase 0 and expires at the Phase 1 switchover, where `ADR-77` (proposed) amends the default. **Why:** Maintainer decision, 2026-11-05 (`MILE-118`); the Decision text is unchanged. | **substantive** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/discovery.rs@sha256:eeef78dc2bb8, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: config.rs still reads the YAML config and init.rs still writes it (ADR-52), discovery.rs still carries no diagnostic for a format the tool no longer reads (ADR-54), header.rs still parses the header as one closed structure and reports duplicate keys (ADR-8), main.rs still hands the paths argument to check (BUG-1), report.rs and main.rs still hold the shared Report and emit contract (BUG-19), the corpus-j renamed-record expectation files still exist (BUG-191), tests/common/mod.rs still owns the TempDir that removes fixture directories and the test-no-leak scripts still fail on a leftover (BUG-193), init.rs and migrate.rs still emit JSON reports (BUG-20), report.rs still declares ScopeSource with the SPEC-2 values and discovery.rs maps to it (BUG-25), and prepare-release.yml still fails loudly with the knope.toml release steps behind it (BUG-27). BUG-2 and BUG-23 are not in this batch because the code they cite moved out of rules.rs. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/discovery.rs@sha256:eeef78dc2bb8c7250e600f40093c5701c08f0de39d70e9f5c075835c8fca665b, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b8337c1172d3c4825859e00a0344dd4c35e6f013d802e509b1688ecb` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` eeef78dc2bb8 -> 5f489efc9a95. Reason: rust/crates/urzua-cli/src/discovery.rs changed only in compute_claim_measures, which now takes the blame line from the shared evidence_anchor_line (the Realized-by line, else the first Done-means item evidence clause) instead of the Realized-by field line; the code this record cites is untouched, so the claim stands (diffed, not re-read in full). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 5f489efc9a95 -> a0044b046d01. Reason: discovery.rs split compute_drifted_records_at into DriftJudge and its helpers, read_claim_files into claim_markdown_paths, and shared one resolved_model; blames go through a per-run BlameCache; behaviour unchanged, outputs byte-identical **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` a0044b046d01 -> 7f85dbcd6026. Reason: Read the diff of each re-pinned file against main. cli/src/discovery.rs: drift and claim measures go through one per-run BlameCache with the drift judge split into helpers, and Notice literals gain detail: None. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 7f85dbcd6026 -> 8ea59ee343dc. Reason: Read the diff of each pinned file since the old pin and the claim still holds. discovery.rs: read_pull_ref_claims' git closure is urzua_io::git::stdout_in (same 'could not run git' / 'git <cmd> failed' wording); nothing else changed; behaviour unchanged, outputs byte-identical. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 8ea59ee343dc -> 8c83e667b37b. Reason: Read the diff of each pinned file or span since the old pin against what this record evidences. rust/crates/urzua-cli/src/discovery.rs: the drift computation judges every locator of a claim instead of stopping at the first drift: DriftedRecords gains a per-record `locators` map, Judged gains a Fresh variant, a new `note` helper records each verdict with its category, and the judge_* helpers return a verdict instead of inserting into the sets; which records are drifted or fresh is unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 8c83e667b37b -> ba6412881ce2. Reason: Read the diff of each pinned file against main. discovery.rs: a held-back drift now carries the time it leaves the grace window and the drift result records it for the notice; which locators drift and which are held back are unchanged; so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` ba6412881ce2 -> e9bea8f7e9a4. Reason: discovery.rs: drift-grace notice wording states the window end in UTC; LocatorDrift no longer carries inside_grace; **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` e9bea8f7e9a4 -> 875922a6e902. Reason: Read the diff of each pinned file against main. rust/crates/urzua-cli/src/discovery.rs: DriftJudge reads the record's Re-verified rows in the tool's exact shape that acknowledged drift and, for a category whose drift policy is acknowledge, treats a change of an unpinned locator that a row names, on or before that row's day, as acknowledged (locator_name, acknowledged_through, acknowledged); compute_base_drift is new and judges the merge base's copy of a record for check --base; note records each drifted locator's name; no other judgement changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 875922a6e902 -> 0a0f6e76f933. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/src/discovery.rs: the acknowledged-row reader skips a Re-verified row dated after the evaluation date (today or --as-of), so a future-dated row clears no drift; no other judgement changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/discovery.rs` 0a0f6e76f933 -> 46fb265a9f06, `rust/crates/urzua-cli/tests/check_integration.rs` 5ab61b95fd58 -> a60c9c61b94c. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/src/discovery.rs: the claim_paths walk matches the md extension case-insensitively; nothing else changed. rust/crates/urzua-cli/tests/check_integration.rs: adds one regression test that a claim file with an upper-case .MD extension under claim_paths is read; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
