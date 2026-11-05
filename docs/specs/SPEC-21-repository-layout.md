---
Version: '0.3'
Date: 2026-09-19
Status: Accepted
Embodiment: Verified
Realized-by: code:Makefile, test:rust/crates/urzua-cli/tests/layout_table_integration.rs
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
| `scripts/` | Repository tooling belonging to no language directory. Currently `generate-dashboard.py` (`ADR-37`), `generate-rule-table.py`, `draft-record.sh`, and `hooks/` (the record-edit hook scripts). |
| `ts/` | TypeScript/Node: agent-harness integrations, editor plugins, dashboard. **Reserved; holds a README only.** |
| `platform/` | Deployment targets and infrastructure, per environment. **Reserved; holds a README only.** |
| `.urzua/` | This repository's own `config.yaml` and record templates -- the tool's configuration of itself. |
| `.github/` | Workflows, `actions/urzua-gate` (the composite gate action), `ISSUE_TEMPLATE/`, `PULL_REQUEST_TEMPLATE.md`, `release-paths` (`ADR-49`), and `scripts/` (the release guard and release invariant scripts, `BUG-48`). |
| `.claude/` | Claude Code settings and the `/adr`, `/rfc` and `/milestone` slash commands for drafting records. |
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
   absent, is a finding against this spec. A test enforces it: `layout_table_integration.rs`
   compares the top-level entries of the table and of the root-files line with `ls -A` of the
   repository root (`.git` excepted) and fails on any entry in one and not the other.
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
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:Makefile, test:rust/crates/urzua-cli/tests/layout_table_integration.rs` -- stated `Verified` disagreed with the computed value. | **structural** |
> | 2026-11-04 | Bumped to `0.2`. The reference to the not-yet-created corpora directory now says it does not exist yet, instead of naming a path that `citation.path-exists` correctly reports as missing. **Why:** `MILE-143` -- the rule is now enabled in this repository. | **structural** |
> | 2026-11-05 | Bumped to `0.3`. Layout table amended to match the tree: added `.claude/`; the `scripts/` row lists `draft-record.sh`, `generate-rule-table.py` and `hooks/` beside `generate-dashboard.py`; the `.github/` row lists `actions/urzua-gate`, `ISSUE_TEMPLATE/`, `PULL_REQUEST_TEMPLATE.md` and `scripts/`. Success criterion 1 is now enforced by `layout_table_integration.rs`, and `Realized-by` cites it instead of the workflow file. **Why:** a re-read found the spec stating `Drift detected` because the table had fallen behind the repository again; the criterion existed but nothing ran it. | **substantive** |
> | 2026-11-05 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:Makefile, test:rust/crates/urzua-cli/tests/layout_table_integration.rs` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
