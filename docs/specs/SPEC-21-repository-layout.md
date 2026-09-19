---
Version: '0.1'
Date: 2026-09-19
Status: Accepted
Embodiment: Verified
Realized-by: code:Makefile, test:.github/workflows/checks.yml
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

| Directory | Contents |
|---|---|
| `rust/` | Cargo workspace: the `urzua` CLI and its crates (`urzua-cli`, `urzua-core`, `urzua-io`, `urzua-id`, `urzua-agdr`). |
| `docs/` | Governance records -- ADRs, RFCs, specs, bugs, milestones, waivers. Language-agnostic by nature; the record format is the contract every language directory implements against. |
| `scripts/` | Repository tooling that belongs to no language directory. Currently `generate-dashboard.py` (`ADR-37`). |
| `ts/` | TypeScript/Node: agent-harness integrations, editor plugins, dashboard. **Reserved, empty.** |
| `platform/` | Deployment targets and infrastructure, per environment. **Reserved, holds a README only.** |
| `.github/` | Workflows, the release-paths manifest (`ADR-49`), and the release guard (`BUG-48`). |

`corpora/` is **not** currently reserved. `ADR-4` named it for `SPEC-4`'s fixtures, which are unbuilt
(`MILE-101`); the directory is claimed when that milestone builds them, and naming it before then is
the speculative reservation this spec exists to stop accumulating.

## Rules

1. **Nothing language-specific at the root.** No `Cargo.toml`, no `package.json`, no `pyproject.toml`.
   The root holds the `Makefile`, `README.md`, `CLAUDE.md`, `AGENTS.md`, `.github/` and the language
   directories.
2. **Every language directory is entered through the root `Makefile`.** Contributors go through the
   root dispatch rather than each ecosystem's native commands, which is `ADR-4`'s accepted cost.
3. **A directory is listed here when it exists**, not when it is anticipated. A reserved-but-empty
   directory is marked as such, so the difference between *planned* and *present* is readable.

## Success criteria

1. This table matches `ls` at the repository root. A directory present and unlisted, or listed and
   absent, is a finding against this spec.
2. `ADR-4` is not edited to track layout changes. It records why the layout is language-first; this
   spec records what the layout is.

## References

- ADR-4 -- the decision, and the table this supersedes in function.
- ADR-37 -- `scripts/generate-dashboard.py`.
- ADR-49 -- `.github/release-paths`.
- SPEC-1 -- restated this layout until 2026-09-19, when it was narrowed to the CLI.
- MILE-101 -- builds `SPEC-4`'s fixtures, and claims `corpora/` when it does.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed at `Version: 0.1`. **Why:** `ADR-4` carried a reserved-names table that had drifted in both directions -- `corpora/` reserved and absent, `scripts/` present and unlisted -- because a frozen decision cannot track a living inventory. Split out when `SPEC-1` was narrowed to the CLI and its restatement of the layout needed a home that was not a second copy. | **substantive** |
