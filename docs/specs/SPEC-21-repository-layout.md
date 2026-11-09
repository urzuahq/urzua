---
Version: '0.5'
Date: 2026-09-19
Status: Accepted
Embodiment: Verified
Realized-by: code:Makefile@sha256:ccc04e61c2e32aad9ccd10166a12df24d31e8ccb1b24233f01c566d7058f382a, test:.github/workflows/checks.yml@sha256:a201f4e57f5f91a353ee0a7449831c0d03263200ac8e83a8f2a7a5b54fbe95a9
Author: beauwilliams
Stable-Id: 01M2VNAMK7V28A67T84JZ5NZPP
Subject: 'What each top-level directory holds, and the rules the root obeys -- the living counterpart to ADR-4''s decision to organize by language.'
Implements: ADR-4
Parent: SPEC-1
---
# SPEC-21 — Repository layout

## Purpose

`ADR-4` decided to organize this repository **by language at the root**, and carried a table of
reserved top-level names to make the decision concrete. An ADR is a frozen, point-in-time decision
fact (`SPEC-18`); a directory listing is current truth that changes as the repository does. Those are
different kinds of statement, and the table has already drifted from both directions:

- **`corpora/` is reserved and does not exist.**
- **`scripts/` exists and is not reserved** -- it holds `generate-dashboard.py`, which `ADR-37` cites.

Neither is a defect in `ADR-4`. The decision it records is unchanged and correct; what aged is the
inventory attached to it, which is what this spec now holds.

## Layout

| Path | Contents |
|---|---|
| `rust/` | Cargo workspace: the `urzua` CLI and its crates (`urzua-cli`, `urzua-core`, `urzua-io`, `urzua-id`, `urzua-agdr`). |
| `docs/` | Governance records -- ADRs, RFCs, specs, bugs, milestones, waivers. Language-agnostic by nature; the record format is the contract every language directory implements against. |
| `scripts/` | Repository tooling belonging to no language directory. Currently `generate-dashboard.py` (`ADR-37`). |
| `ts/` | TypeScript/Node: agent-harness integrations, editor plugins, dashboard. **Reserved; holds a README only.** |
| `platform/` | Deployment targets and infrastructure, per environment. **Reserved; holds a README only.** |
| `.urzua/` | This repository's own `config.yaml` and record templates -- the tool's configuration of itself. |
| `.github/` | Workflows, `release-paths` (`ADR-49`), and the release guard script (`BUG-48`). |
| `.changeset/` | Pending release fragments (`ADR-29`), consumed by `knope` at release time. |
| `.git-hooks/` | The pre-commit and pre-push hooks installed by `make hooks-install`. |

Root files: `Makefile`, `README.md`, `AGENTS.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
`SECURITY.md`, `LICENSE`, `CHANGELOG.md`, `knope.toml`, `.coderabbit.yaml`, `.gitignore`.

`corpora/` is **not** currently reserved. `ADR-4` named it for `SPEC-4`'s fixtures, which are unbuilt
(`MILE-101`); the directory is claimed when that milestone builds them, and naming it before then is
the speculative reservation this spec exists to stop accumulating.

## Rules

1. **Nothing language-specific at the root.** No `Cargo.toml`, no `package.json`, no `pyproject.toml`.
   The root holds the files listed above, the dot-directories, and the language directories.
2. **Every language directory is entered through the root `Makefile`.** Contributors go through the
   root dispatch rather than each ecosystem's native commands, which is `ADR-4`'s accepted cost.
3. **A directory is listed here when it exists**, not when it is anticipated. A reserved-but-empty
   directory is marked as such, so the difference between *planned* and *present* is readable.

## Success criteria

1. This table matches `ls -A` at the repository root -- `-A`, because half the entries it
   lists are dot-directories that plain `ls` hides, which is how the first version of this table
   shipped omitting three of them. A directory present and unlisted, or listed and
   absent, is a finding against this spec.
2. `ADR-4` is not edited to track layout changes. It records why the layout is language-first; this
   spec records what the layout is.

## References

- ADR-4 -- the decision, and the table this supersedes in function.
- ADR-37 -- `scripts/generate-dashboard.py`.
- ADR-49 -- `.github/release-paths`.
- SPEC-1 -- restated this layout until 2026-09-19, when it was narrowed to the CLI.
- MILE-101 -- builds `SPEC-4`'s fixtures, and claims a `corpora` directory, which does not exist yet, when it does.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed at `Version: 0.1`. **Why:** `ADR-4` carried a reserved-names table that had drifted in both directions -- `corpora/` reserved and absent, `scripts/` present and unlisted -- because a frozen decision cannot track a living inventory. Split out when `SPEC-1` was narrowed to the CLI and its restatement of the layout needed a home that was not a second copy. | **substantive** |
> | 2026-09-19 | Table corrected against `ls` on the day it landed. **Why:** it failed its own success criterion 1 immediately -- `.urzua/`, `.changeset/` and `.git-hooks/` existed and were unlisted, `ts/` was called empty while holding a README, and rule 1 named a `CLAUDE.md` that is not in this repository. Caught by review, which is the second time a record has been filed asserting something the working tree contradicts; the criterion is now checkable rather than aspirational. | **substantive** |
> | 2026-09-19 | Success criterion 1 names `ls -A`, not `ls`. **Why:** plain `ls` hides `.urzua/`, `.changeset/`, `.github/` and `.git-hooks/` -- four of the nine entries this table lists -- so the criterion could not have detected the omission it was written to prevent, and did not. | **substantive** |
> | 2026-11-04 | `Embodiment` recomputed by `urzua fix`: `Drift detected`. **Why:** every file named in `Realized-by` has changed since this line was last verified, and this pass did not re-verify it, so the record states the drift instead of a stale `Verified`; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:Makefile, test:.github/workflows/checks.yml` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.2`. The reference to the not-yet-created corpora directory now says it does not exist yet, instead of naming a path that `citation.path-exists` correctly reports as missing. **Why:** `MILE-143` -- the rule is now enabled in this repository. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: Makefile@sha256:fb9fda4c8508, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: config.yaml and the own-config test still declare the adr, rfc, spec and condition types that SPEC-16, SPEC-17, SPEC-18 and SPEC-23 describe, with spec_shipped_integration.rs, rules/roadmap.rs, condition_type_integration.rs and work_types_integration.rs behind them, check.rs, rules.rs, check_integration.rs and fail_on_integration.rs still implement urzua check (SPEC-2), the workflows, the install-knope action and the release and gate test scripts still make up the pipeline SPEC-20 describes, the Makefile and checks.yml still dispatch the repository layout of SPEC-21, the harness modules, the bundled hook script and the harness integration tests still implement the request, verdict and adapter layout of SPEC-24, config.rs and the own-config and field-levels tests still implement SPEC-3, and acceptance.rs and acceptance_property.rs still implement SPEC-4. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Embodiment` from `Realized-by: code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436, test:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-08 | Bumped to `0.3`. The `code:` and `test:` locators of `Realized-by` are now hash-pinned by `urzua reverify --pin-only` (the `Pinned` row above) and, where the record stated `Drift detected`, its `Embodiment` was recomputed (the tool-authored row above); no behaviour this spec describes changes. **Why:** `MILE-129`, `ADR-97` section 10: a claim is renewed only over pinned code and test evidence, so the corpus is pinned once before expiry is enabled. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `Makefile` fb9fda4c8508 -> ccc04e61c2e3, `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. Makefile: the rust-test target now runs the tests through .github/scripts/ci-tests.sh (nextest, doctests and the acceptance runner under the leak guard) and the Makefile gained the ci-gate-test and nextest-pin targets, and the targets the record relies on are unchanged; checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.github/workflows/checks.yml` 3bd0a6795184 -> a201f4e57f5f. Reason: checks.yml: the only change since the pinned version is the four tool-download curl lines (cargo-deny, cargo-machete), which gained --retry 5 --retry-all-errors --retry-delay 5 and changed -sSLfo to -sSLf with a separate -o (#441); no step, job or check changed, so the evidence still holds **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Bumped to `0.5`. Re-pin rows in this revision log were reworded to name who re-verified, matching the tool's wording since 2026-11-09; hashes, reasons and dates are unchanged. **Why:** Maintainer decision, 2026-11-09. | **structural** |
