---
Stable-Id: 01M6R3NK4CFP9CSWVTMWMZ9460
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-cli/src/main.rs@sha256:0b6a713c8273052cddbe0e73918859a4b0ff6d2c5e51e66029f32e9d73ffcce3, test:rust/crates/urzua-cli/tests/check_integration.rs@sha256:5ab61b95fd5864a41566dee0d902fac000dca3552f90316331ecf90617c3f9b0
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
