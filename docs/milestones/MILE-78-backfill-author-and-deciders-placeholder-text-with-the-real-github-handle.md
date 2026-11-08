---
Status: Done
Stable-Id: 01M1Z1EP2A91E5255WDRGQPYGD
Phase: '0'
Track: accountability-identity
Implements: —
Blocked-on: —
Realized-by: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e7dbcfee70b6c994f022fb02a22b138bea50167028e46c232ceed, test:rust/crates/urzua-core/src/field_state.rs#retired_project_placeholder_conventions_are_placeholder_not_present@sha256:93bd89a306f7a4462d5e6d89612bb354976646abfd0502a4018b5934a54ae327, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#an_accepted_adr_whose_deciders_is_a_declared_placeholder_is_reported@sha256:d9525ed1a224569a925e8cf8bb4e889a07a334a1de25a658018bf39c901ce26f, code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436
---
# 78 — Backfill Author and Deciders placeholder text with the real GitHub handle

## What

Replace every anonymized `Author`/`Deciders` placeholder (the literal text this project used before
identity was resolved) across the corpus with the real GitHub handle, `@beauwilliams` -- 92
instances across ADRs, RFCs, and specs. Distinct from MILE-18 (real identity *resolution* as a
mechanism, e.g. via `fix --apply`'s existing identity-resolution path for Reviewer/Decider roles):
this is a corpus data fix with the real value known today, not a tooling build. `urzua new` already
resolves a real identity for newly created records (ADR-27) -- this backfills everything authored
before that, plus one live regression (ADR-38, written this session, where the placeholder was
typed by hand over the tool's own auto-filled value).

## Why

Found live: reviewing ADR-38, the anonymized placeholder appeared in `Author`/`Deciders` again
despite `urzua new` already resolving the real identity automatically -- a hand-edit had silently
overwritten it. Checking the rest of the corpus found the same placeholder in 92 places going back
to the first RFC. A real decision record's `Author`/`Deciders` accountability is the entire point of
requiring the field (RFC-1) -- an anonymized placeholder in every single one defeats that as
thoroughly as an unfilled blank would, just less visibly, since `field.quality` doesn't flag
placeholder-shaped prose that happens to already be non-empty.

## Done means

- No ADR, RFC or spec in this repository carries the anonymized `Author` or `Deciders` placeholder; every one names a real GitHub handle. — evidence: code:.urzua/config.yaml@sha256:151830e5926d1887be39a723d110744ff5e2ae8c408bb114a90ba3e2b8316a2e, code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d
- `field.quality` and `field.pending` (`field_state.rs`) report a placeholder if one returns. — evidence: code:rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e7dbcfee70b6c994f022fb02a22b138bea50167028e46c232ceed, test:rust/crates/urzua-core/src/field_state.rs#retired_project_placeholder_conventions_are_placeholder_not_present@sha256:93bd89a306f7a4462d5e6d89612bb354976646abfd0502a4018b5934a54ae327, test:rust/crates/urzua-cli/tests/placeholder_vocabulary_integration.rs#an_accepted_adr_whose_deciders_is_a_declared_placeholder_is_reported@sha256:d9525ed1a224569a925e8cf8bb4e889a07a334a1de25a658018bf39c901ce26f
- `urzua check` over this repository reports no such placeholder. — evidence: code:.github/workflows/checks.yml@sha256:4afa5fe0db59c78f2954f105354af9f42c4fef044c7a0e960384c59aa880313d, code:Makefile@sha256:fb9fda4c85083447873cd372df238eae66f507575c2257c8f275f2253e4ae436

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Fixed and shipped: mechanically replaced all 92 placeholder instances (`Author`/`Deciders`) with `@beauwilliams` across every ADR, RFC, and spec. **Why:** the placeholder defeated the actual purpose of the `Author`/`Deciders` fields (RFC-1's accountability requirement) in every single record that had one; the real value has been known the whole time, so leaving it unfixed was pure debt, not an open question. | **substantive** |
> | 2026-11-04 | `Realized-by` added; `Status` stays Done. **Why:** `status.delivery-agreement` (`MILE-183`) asks a `Done` milestone for delivery evidence; the locators name the code, tests or decision record that deliver it, taken from the changes that built it. | **structural** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. The criteria are those the delivered work met, checked against `Realized-by` and the code; `Status` stays Done. | **structural** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 4 locators pinned, 4 file-level, 0 scoped. Evidence: .urzua/config.yaml@sha256:151830e5926d, .github/workflows/checks.yml@sha256:4afa5fe0db59, rust/crates/urzua-core/src/field_state.rs@sha256:d46c56def01e, Makefile@sha256:fb9fda4c8508 Reason: Pinned the files as they are now after checking each against the record it evidences: section.rs, document_model.rs, table.rs and config.yaml still hold the decisionless-status exemption (MILE-7), config.yaml and the milestone template still carry the WontDo terminal status (MILE-71), the spec template and config.yaml still make urzua new spec work (MILE-74), field_state.rs, config.yaml, checks.yml and the Makefile still report a returning placeholder (MILE-78), config.yaml still declares the milestone type (MILE-8), table.rs, pointer.rs, commands/rules.rs, generate-rule-table.py and config.yaml still give every rule a configurable level (MILE-80), config.yaml still declares the per-type field sets (MILE-81), config.yaml, pointer.rs and narrative.rs still hold the opt-in narrative-field.stale rule (MILE-83), schema_version.rs, the corpus-a expectation and config.yaml still show the removed shapes refused (MILE-85), and new.rs, new_refs.rs, open_pr.rs, report.rs, config.rs, discovery.rs, check.rs, config.yaml and table.rs still hold the open pull request numbering check (MILE-89). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
