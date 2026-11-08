---
Status: Done
Stable-Id: 01M1Y5JDMZ6T2KYMZZC2HKP78T
Phase: '0'
Track: header-format
Implements: ADR-33
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089, code:.github/workflows/checks.yml@sha256:3bd0a6795184fddb53765966c82487ac9f3edf170cbb05df5b58683a6cf43a8b
---
# 3 — Convert this repo's own docs to yaml-frontmatter

## What

A one-time, unshipped conversion pass over this repo's own 166 ADR/RFC/Spec/Milestone/Bug records
(all six configured types per ADR-33's amendment; `waiver` has none yet, so only its config
declaration changed), byte-preserving everything below the header.

## Why

The actual dogfooding step ADR-33 named as the real forcing function for this work -- no adopter is under pressure to migrate, so this repo doing it first is the only evidence the mechanism is safe.

## Done means

- Every record type configured in this repository's `.urzua/config.yaml` declares `header_shape: yaml-frontmatter`. — evidence: code:.urzua/config.yaml@sha256:0b59c6dfc8b9eeb00bdfd4ed9a44b778807a4a182f115720a8948c9f982bd089
- Every ADR, RFC, spec, milestone and bug record parses under it, and `urzua check` over this repository reports no unreadable header. — evidence: code:.github/workflows/checks.yml@sha256:3bd0a6795184fddb53765966c82487ac9f3edf170cbb05df5b58683a6cf43a8b
- The conversion was a one-time pass that shipped no tool, and nothing below the header changed. — waived: process-outcome -- a property of the one-time conversion of 2026-09-08, verified at the time by a before/after field-set comparison against every file (see the revision log); the scratch tool was never committed, so no file can evidence it now

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Cleared `Blocked-on` (was `BUG-3`, `Status: Fixed` for a while, caught by `blocked-on.stale` -- MILE-83 -- immediately after `Blocked-on` moved from prose into a checked header field). **Why:** the blocker had silently resolved and nobody had revisited this milestone since; found live during backlog triage, confirmed by the new rule firing on this exact case before this fix. | **substantive** |
> | 2026-09-08 | `Status: Done`; scope corrected from "~35 ADR/RFC/Spec/Bug/Milestone" to the actual 166 files across all six configured types (ADR-33's amendment). Migrated via a scratch, uncommitted Rust tool reusing `header::parse_with_shape` -- not shipped, per ADR-33's explicit "not a new public subcommand" requirement. Verified by a real before/after field-set-equality comparison against every file (zero mismatches), `check docs/` (zero blocking findings, `header.deprecated-shape`/`type.no-declared-spec` both silent), `graph` (zero dangling edges), and a dashboard regeneration confirming real field values render instead of `—`. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, .github/workflows/checks.yml@sha256:4afa5fe0db59 Reason: Pinned the files as they are now after checking each against the record it evidences: fix.rs and the changeset still carry the attestation identity source and actor kind into the revision row (MILE-201), settings.json still runs check after a Bash command (MILE-202), config.rs, report.rs, table.rs and config.yaml still carry the scope: changed option and its deferred-by-scope count (MILE-203), settings.json and urzua-claude-code.py still run on the installed harness bundle (MILE-204), config.rs, file_texts.rs, session.rs and init.rs still read discovery.include_untracked (MILE-206), config.rs, revision_log.rs, both fix files and the corpus-w and corpus-zero expectations still read the declared class vocabulary and column (MILE-213), delivery_items.rs, status.rs, fix.rs, urzua-record.sh, done.md and the corpus-v files still compute a milestone's status from its items (MILE-214), the corpus-zero expectation files and checks.yml still gate the migrated corpus (MILE-215), config.yaml, sequence.rs and relation.rs still declare and check the acyclic Depends-on pair (MILE-27), and config.yaml still declares every type as yaml-frontmatter with checks.yml gating it (MILE-3). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml` 151830e5926d -> 0b59c6dfc8b9, `.github/workflows/checks.yml` 4afa5fe0db59 -> 3bd0a6795184. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml: config.yaml gained Kind among the milestone type's known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and every other declaration the record relies on is unchanged; checks.yml: the single rust job became parallel lint, build, three nextest test shards, four acceptance shards, an acceptance-gate and a result job (ADR-98), and every step the record relies on (fmt, clippy, deny, the records gate, the release guards, the rule table check, the gate action tests and harness status) is still run. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
