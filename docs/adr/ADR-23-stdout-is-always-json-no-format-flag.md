---
Stable-Id: 01M6R3NK4CFP9CSWVTMWMZ9460
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs@sha256:1113ab988c141bc174343fa8ff6b48dd81b93352aa3f1f042fbfc24000f358cd, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:bc2e870defc676cbc3c053ab78b577276cf7696b10c6ef586be3ee37548abf64
Date: 2026-09-06
Accepted-on: 2026-09-06
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: ADR-7
Derives-from: RFC-3
---
# 23 — Stdout is always JSON; no `--format` flag

## Context

RFC-3's actual proposal was that stdout is *unconditionally* the JSON report — "every entry
point's final, primary output on stdout is exactly one JSON object, always," with human-readable
prose moved to stderr entirely, no flag deciding between them. ADR-7 narrowed that at
implementation time to an opt-in `--format json` flag, human-readable by default — reasonable for a
CLI unsure yet who'd actually run it.

That premise no longer holds. Urzua is not a linter a human runs and reads; it's a structured record
engine whose primary consumer is an agent, and a human's access to that data goes through an agent
reading it, not through squinting at terminal output directly. A tool built entirely around
structured records should not make its own primary interface unstructured by default.

## Decision

In the context of a fully agent-driven tool, facing ADR-7's default working against the actual
audience, we decided **stdout is always the JSON report, unconditionally — no `--format` flag, no
opt-in, no human default.** `check`, `fix`, and `migrate schema --report` print exactly one JSON
object to stdout on every invocation. The human-readable rendering — the same one `--format human`
used to produce — now prints to stderr, always, on every invocation too. A person running the tool
directly in a terminal sees both streams (they render to the same terminal by default); a consumer
that captures stdout alone sees JSON only, unconditionally, without having to know to ask for it.

`OutputFormat` and every `--format` argument on `check`/`fix`/`migrate schema` are removed, not
deprecated — there is no flag to reintroduce this ADR was written to eliminate.

## Reversibility

Same reversibility profile ADR-7 already named: this is the output contract anything scripting
against `urzua` depends on. Cheap now — no known external consumer yet — expensive once one exists,
which motivates deciding this now rather than after v1.0.0.

## Consequences

- Every future command's default output changes shape: JSON on stdout is not something a caller
  opts into, it's what's already there.
- **Stale as of ADR-46**: `urzua doctor` now follows this contract fully (it did not when this ADR
  shipped). `urzua init` and `urzua migrate ids` still don't — both are plain-text prose end to end,
  a materially different redesign than a drop-in `Report` struct. Filed as its own bug rather than
  fixed piecemeal.
- ADR-7's "human-readable format is not contractual" clause still holds, now for the stderr
  rendering specifically.
- **Narrowed by ADR-26**: the stderr human-rendering clause in this decision (the paragraph above
  describing it) was removed shortly after this ADR shipped — there is no second rendering at all,
  on stderr or anywhere else. Everything else in this ADR (stdout is always the JSON report,
  unconditionally, no `--format` flag) stands unchanged.
- **Extended by ADR-46**: the one shape every command shares is now concrete (`Report`/`Notice`/
  `emit()`), not just an aspiration this ADR names.

## References

- ADR-7 — the narrower decision this supersedes.
- ADR-46 — the concrete shared shape (`Report`/`Notice`/`emit()`) this ADR's "same shape" goal names
  but doesn't itself specify.
- RFC-3 — the original proposal this restores.
- `rust/crates/urzua-cli/src/main.rs` — `emit()`, and every `run_*` function's output path.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/main.rs, test:rust/crates/urzua-cli/tests/check_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 4435810c0. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf84, rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b833 Reason: Pinned the files as they are now after checking each against the record it evidences: header.rs still parses the declared YAML frontmatter shape (ADR-17), fix.rs still has detect_repairs (ADR-19), urzua-id still builds ULID stable ids (ADR-21), main.rs still prints JSON with no --format flag on check (ADR-23), urzua-io still resolves the attesting identity in the ADR-31 order, scripts/generate-dashboard.py still generates the static dashboard (ADR-37), report.rs still defines Report and Notice and main.rs still emits them (ADR-46), publish-release.yml still verifies the tree it ships through checks.yml (ADR-47), knope.toml still releases from changesets (ADR-48), and ci.yml still gates changesets on .github/release-paths (ADR-49). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-cli/src/main.rs@sha256:9fae4c3edf849d76d0f53912bc9c090f34f0666ef1a85b2a002b29747fe2ea14, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:f893cb99b8337c1172d3c4825859e00a0344dd4c35e6f013d802e509b1688ecb` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 9fae4c3edf84 -> f317c199a33d. Reason: Read the diff of each pinned file since the old pin and the claim still holds. main.rs: only the doc comment on the Init command changed, so its --help text now says --dry-run prints the config the real run would write instead of calling it byte-identical to the real run; no flag, argument or behaviour changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` f893cb99b833 -> 8538aedb7492. Reason: the tests in check_integration.rs now name real tracked files; the behaviour this record relies on is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 8538aedb7492 -> 5ab61b95fd58. Reason: check_integration.rs: the only change is in pre_commit_hook_stops_a_planted_bad_header_with_the_real_binary, which now symlinks the urzua binary into the fixture instead of copying it (a copy raced a forking test thread and failed with ETXTBSY under load); the test's setup, planted defect and assertions are otherwise unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` f317c199a33d -> e3b1b6050ff1. Reason: Read the diff of each re-pinned file. main.rs: added the global --as-of option on Cli, the fix_evaluation_date gate that refuses it with --apply and on commands that compare no dates, the as_of() report helper, and parse_cli, which now wraps the unchanged argument parse and that gate; command dispatch otherwise unchanged. Nothing this record relies on changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` e3b1b6050ff1 -> c7e27293c39c. Reason: Read the diff of each re-pinned file against main. main.rs: the --help about text now carries the one-line description stated in RFC-27 (ending 'It never judges the change.'); no command or option changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` c7e27293c39c -> 0b6a713c8273. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: main.rs: split main into install_panic_hook, panic_if_requested, dispatch_migrate, dispatch_harness and dispatch_export to stay under the SPEC-22 function bound; main's #445 --help about text is unchanged. Behaviour unchanged, outputs byte-identical (check, check --base origin/main, fix, fix --rebaseline, reverify, reverify --as-of 2027-06-01 and --help stdout, stderr and exit compared against a binary built from origin/main). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` 5ab61b95fd58 -> a60c9c61b94c. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-cli/tests/check_integration.rs: adds one regression test that a claim file with an upper-case .MD extension under claim_paths is read; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 0b6a713c8273 -> bb8f51c2f61e. Reason: Read the diff of each re-pinned file against main. main.rs gained a Waive variant of the Command enum (with its --from-report, --rule, --path and --by flags), one dispatch arm calling commands::waive::run and one command_name arm; every existing variant and arm is unchanged. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` bb8f51c2f61e -> 6aea9772c936. Reason: Read the diff of each re-pinned file against main. since the previous pin of this change: the Waive dispatch arm passes rule, path and by straight to commands::waive::run instead of building an args struct in main (main was over the function-size bound); the Command enum and every other arm are unchanged, no behaviour changed. [first delta of this change: main.rs gained a Waive variant of the Command enum (with its --from-report, --rule, --path and --by flags), one dispatch arm calling commands::waive::run and one command_name arm; every existing variant and arm is unchanged]. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 6aea9772c936 -> fbc030ae62bb. Reason: Read the diff of main.rs against main: it adds the migrate filenames subcommand variant with its doc comment and one dispatch arm; the rest of the file is unchanged, and the behaviour this record relies on is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` fbc030ae62bb -> 8e0accde933c. Reason: Re-pinned after PR4 of MILE-205 (urzua waive --apply), the stated delta being: main.rs: `Command::Waive` holds a clap WaiveArgs struct (defined in commands/waive.rs) that gains `--apply`, `waive` joins the `--as-of` refusal match and its message, and the waive arm of main() is one line to stay under the SPEC-22 bound; no other command or output changed. The claim holds as the changed code does not alter what this record states. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 8e0accde933c -> 4f82d38719b2. Reason: Read the diff of main.rs against main. It gained one Mcp command variant (urzua mcp, with --root), its dispatch arm and its name in command_name; every existing variant and arm is unchanged, so the claim this record evidences still holds. The new command serves JSON-RPC frames on stdout rather than one emit() envelope, which this record's claim does not cover. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 4f82d38719b2 -> d910dde60ed4. Reason: Read the diff of main.rs (and the Command enum in SPEC-1's pin) against main: the Init command gained the --types and --dir options with their help text and reaches init::run through a new dispatch_init; the Mcp command main added is untouched. No other command or option changed. The claim each record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` d910dde60ed4 -> 50769555b512. Reason: Read the diff of main.rs against main. The command match moved unchanged out of main() into a new dispatch() function so main() stays short; the Mcp variant and its arm were already on main from #492, and every variant and arm is unchanged, so the claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/tests/check_integration.rs` a60c9c61b94c -> bc2e870defc6. Reason: Read the diff of each re-pinned file against main and the claim still holds. check_integration.rs: the ADR-0001 waiver fixture's H1 gains the citation-fixture marker text and every_header_reading_rule_reports_a_header_none_type_as_out_of_scope declares the new rule's required options; no assertion changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 50769555b512 -> c732584d7812. Reason: Read the diff of rust/crates/urzua-cli/src/main.rs against main. emit now serializes through a private WithNextAction wrapper that appends next_action and next_action_reason after the report's own keys, and falls back to a could-not-run JSON with exit 2 when a report cannot serialize as an object; report_value (used by urzua mcp) is added; the panic hook JSON gains both keys as null (MILE-72). emit still writes one JSON document and otherwise returns the report's exit code, so the claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` c732584d7812 -> a305b2779ee7. Reason: Read the diff of rust/crates/urzua-cli/src/main.rs against main: only help text changed. The doc comment of the New command now says the skipped numbers are in the JSON skipped field (stderr stays empty of them), and the --as-of help names waive among the commands that accept it. No command, flag, output key or behaviour changed, so each claim this record makes about main.rs (and SPEC-1's about the Command enum) still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` a305b2779ee7 -> 493de1549bc5. Reason: Read the diff of rust/crates/urzua-cli/src/main.rs against main: only help text changed. The doc comment of the New command now says the skipped numbers are in the JSON skipped field and not on stderr, and the --as-of help names waive among the commands that accept it. No command, flag, output key or behaviour changed, so each claim this record makes about main.rs (and SPEC-1's about the Command enum) still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-cli/src/main.rs` 493de1549bc5 -> 1113ab988c14. Reason: Pinned file(s) changed by the symlinked-record refusal: main.rs: adds the NOTICE_ESCAPING_RECORDS subject constant. The record's claim is unaffected by these changes. (Re-applied after merging main.) **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
