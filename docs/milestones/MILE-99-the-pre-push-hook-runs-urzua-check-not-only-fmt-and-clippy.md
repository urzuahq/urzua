---
Status: Done
Stable-Id: 01M2VN4C5S5YSDEHK5P136XBAQ
Phase: '0'
Track: governance-process
Implements: SPEC-1
Blocked-on: —
Realized-by: code:.git-hooks/pre-push@sha256:2e01e0e6b77f87884289ac46630dcee53c8363350f531764b90931616debde43, code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a, code:.github/workflows/checks.yml@sha256:0a3547c3f12a51cefe3dcc88696bacc8f9911356a4c6e61c56f8ef25e434e493, test:rust/crates/urzua-cli/tests/fail_on_integration.rs#config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing@sha256:d1e8e19c3842276d697a34cbf386f69ca04921417026f4d0cc1a5f9a86219b04
---
# 99 — The pre-push hook runs `urzua check`, not only fmt and clippy

## What

`make hooks-install` installs a pre-push hook that runs `cargo fmt` and `cargo clippy`. It does not
run `urzua check`, so the tool that governs this corpus is enforced in CI and nowhere else.

## Why it is Phase 0

`SPEC-1`'s first success criterion is explicit: *"It runs green in at least two real target codebases
... **via pre-push hook and CI**"*. Half of that is built. The hook is not a convenience -- it is named
in the bar Phase 0 exits on.

It also closes a real loop. A record written on a branch is checked only after it reaches a pull
request, so the feedback arrives once the work is packaged rather than while it is being written. Two
records were pushed this week with headers `check` rejected, and both round-trips would have been
avoided locally.

## The decision this needs

**What the hook does with a non-blocking finding.** The corpus currently reports 61 findings at
`blocking: false`; a hook that refuses a push on any finding would refuse every push today. Options,
not yet chosen:

- Block only on `blocking: true`, matching what CI does -- simplest, and the hook adds nothing CI does
  not already catch.
- Block on any *new* finding relative to the merge base -- catches regressions without demanding the
  backlog be cleared first, and needs a baseline `MILE-49` is already about.

The second is more useful and more work. This milestone owns choosing, not both.

## Not the same as making the corpus clean

61 findings at `warn` is a separate question. This milestone is about *where* the check runs, not about
what level the rules are declared at.

## Done means

- `.git-hooks/pre-push` runs `cargo fmt --check`, `cargo clippy` and the records gate (it builds the release binary and runs `urzua check`). — evidence: code:.git-hooks/pre-push@sha256:2e01e0e6b77f87884289ac46630dcee53c8363350f531764b90931616debde43
- The hook is installed by `make hooks-install`. — evidence: code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a
- `make records` builds the release binary and runs `urzua check` exactly as CI does, so a record `check` rejects does not reach a push. — evidence: code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a, code:.github/workflows/checks.yml@sha256:0a3547c3f12a51cefe3dcc88696bacc8f9911356a4c6e61c56f8ef25e434e493
- The hook acts on what `check` returns under the configured `fail_on`. — evidence: test:rust/crates/urzua-cli/tests/fail_on_integration.rs#config_fail_on_warning_makes_a_warning_exit_1_with_unchanged_json_observed_failing@sha256:d1e8e19c3842276d697a34cbf386f69ca04921417026f4d0cc1a5f9a86219b04, code:.git-hooks/pre-push@sha256:2e01e0e6b77f87884289ac46630dcee53c8363350f531764b90931616debde43

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | Verified against `.git-hooks/pre-push`: it runs `cargo fmt --check`, `cargo clippy` and then `make records`, which builds the release binary and runs `urzua check` exactly as CI does (#134). The decision this record owns is thereby made in code: the hook acts on what `check` returns under the configured `fail_on: warning`, as CI does, with no merge-base baseline, and the corpus now reports no unwaived findings, so the 61 non-blocking findings that motivated the question are gone. The hook installs only through the opt-in `make hooks-install`, and the commit hook exits 0 when the binary is not built; making the hooks reach every contributor and verifiable by `urzua doctor` is `MILE-179`, and `ADR-71` records the layering. Milestones carry no declared evidence field on `main` yet, so the evidence is cited here and `Status` stays `Planned` for the maintainer to close once one lands. **Why:** the milestone is stale, not open work: its title is delivered, and what remains belongs to `MILE-179`. | **substantive** |
> | 2026-09-19 | Filed. **Why:** `SPEC-1`'s first success criterion names the pre-push hook alongside CI, and only CI was built. Surfaced by asking what Phase 0 actually exits on rather than reading the milestone list, which did not contain this. | **substantive** |
> | 2026-11-04 | `Realized-by` added and `Status` Planned -> Done. **Why:** the work is merged in #134: the pre-push hook runs `make records`, which runs `urzua check`. Checked against this record's Done means and the code; the record never carried its own delivery evidence, and `status.delivery-agreement` (`MILE-183`) reads evidence under a not-done `Status` as a disagreement. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 3 file-level, 0 scoped. Evidence: .git-hooks/pre-push@sha256:2e01e0e6b77f, Makefile@sha256:fb9fda4c8508, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: declaration.rs and config.yaml still declare pointer_fields and narrative_fields per type (MILE-90), config.yaml still requires Subject on specs with checks.yml gating it (MILE-91), document_model.rs, both section.rs files, schema_version.rs and config.yaml still read the declared identity, fields and sections (MILE-98), .git-hooks/pre-push, the Makefile and checks.yml still run urzua check (MILE-99), waiver.rs, rules/waiver.rs and waiver_stale_integration.rs still implement the waiver type SPEC-10 describes, audit.rs and the own-config and check integration tests still implement SPEC-11, new.rs, new_record.rs and new_integration.rs still implement SPEC-12, explain.rs, both graph.rs files and the own-config test still implement SPEC-13, both migrate.rs files and check_integration.rs still implement SPEC-14, and doctor.rs and its tests still implement SPEC-15. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `Makefile` fb9fda4c8508 -> ccc04e61c2e3, `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. Makefile: the rust-test target now runs the tests through .github/scripts/ci-tests.sh (nextest, doctests and the acceptance runner under the leak guard) and the Makefile gained the ci-gate-test and nextest-pin targets, and the targets the record relies on are unchanged; checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 3bd0a6795184 -> a201f4e57f5f. Reason: checks.yml: the only change since the pinned version is the four tool-download curl lines (cargo-deny, cargo-machete), which gained --retry 5 --retry-all-errors --retry-delay 5 and changed -sSLfo to -sSLf with a separate -o (#441); no step, job or check changed, so the evidence still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-10 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` a201f4e57f5f -> ad6ab57882df. Reason: checks.yml gained one additive, report-only step in the build job (no stale evidence pins: runs fix --rebaseline without --apply and fails unless the report status is ok); no existing step changed, so the claim still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-14 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` ad6ab57882df -> 0a3547c3f12a. Reason: Read the diff of checks.yml since the pin: the only change is one env entry on the test job, URZUA_BUG_LISTS_MAY_GROW, set from the bug-lists-grow pull request label for acceptance_bug_gate.rs; no step, job, trigger or ordering changed, so the claim about the workflow still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
