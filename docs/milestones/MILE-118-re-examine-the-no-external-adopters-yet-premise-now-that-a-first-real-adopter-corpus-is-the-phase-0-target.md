---
Stable-Id: 01M6CXZV44D4GZWR1D3KT6PZ3J
Status: Done
Phase: '0'
Track: governance-process
Realized-by: code:AGENTS.md@sha256:4260777828689cf9fdfc871943e6dc4b58b66b53053087d23da1b01b1ac2962e, code:rust/crates/urzua-core/src/config.rs@sha256:921027c81b7ecf7d56c357dd548fa28c47c785c7cd322c261c14e0b46626b296, spec:docs/adr/ADR-44-pointer-and-narrative-fields-are-config-declared-per-type-not-hardcoded.md, spec:docs/adr/ADR-54-a-break-is-a-break-no-migration-diagnostics-for-formats-we-no-longer-read.md, spec:docs/rfc/RFC-23-config-declared-pointer-and-blocking-fields-instead-of-hardcoded-in-urzua-core.md, spec:docs/rfc/RFC-26-records-not-github-issues-close-the-issue-tracker-as-a-tracking-surface.md, spec:docs/rfc/RFC-38-a-read-only-subset-does-not-earn-a-command-fold-audit-into-check-and-stop-re-deriving-fix-s-detection.md
---
# 118 — Re-examine the 'no external adopters yet' premise now that a first real adopter corpus is the Phase 0 target

## What

Re-examine the premise that this tool has no external adopters, now that a first real adopter corpus
is the Phase 0 target (`MILE-102`). List every place that relies on it, decide which still hold, and
record the outcome. Every statement below was checked by grep against this branch:

- `AGENTS.md`, "Don't build speculative capability": "this tool has no external adopters yet whose
  behavior needs preserving".
- `rust/crates/urzua-core/src/config.rs` (doc comment near line 193): "no real external adopters yet
  whose behavior a hardcoded fallback would need to preserve".
- `ADR-44` (Accepted): "zero real external adopters" as the stated cost of dropping the default, and
  "imagined future adopters, not real ones" as the rationale for the shim it removed.
- `ADR-54` (Accepted): the removed migration notice "served a population of zero", and a future break
  "with real adopters behind it is a different question".
- `RFC-23` (Accepted): "this tool has no real external adopters yet to preserve".
- `RFC-38` (Draft): quotes the `config.rs` comment as the reason this is "the cheapest moment".
- `RFC-26` (Draft): "External adopters do not exist yet (AGENTS.md's 'no external adopters' premise)".

`ADR-33` and `ADR-61` also say "no adopter" but about pressure to migrate and about declared fields
respectively, not the premise itself, so they are not counted.

## Why

Several removals and non-compatibility decisions were justified by a population of zero. The Phase 0
target is a first real adopter corpus, so the premise is about to be false, and each dependent
decision should be re-read against a nonzero population before the next break is made on its strength.
The two Accepted ADRs and the Accepted RFC listed above are not edited: per `AGENTS.md` ("Never silently
rewrite an Accepted decision") any correction goes in a dated note in their Consequences or in a new
record that narrows them. The Draft RFCs, `AGENTS.md` (through `SPEC-19`) and the `config.rs` comment
can change in place.

## Outcome

Decided by the maintainer on 2026-11-05: the premise that this tool has no external adopters holds through Phase 0 and expires at
the Phase 1 switchover (`MILE-102`), when a real adopter's corpus exists and `RFC-49`'s compatibility contract (proposed in
`ADR-77`) takes over. Each place the What lists keeps its wording and carries a dated note saying so; none is rewritten, and no
Accepted record's decision text is edited.

| Place | Outcome |
|---|---|
| `AGENTS.md`, "Don't build speculative capability" | Kept; dated note added after the sentence |
| `rust/crates/urzua-core/src/config.rs`, `pointer_fields` doc comment | Kept; dated note added to the comment |
| `ADR-44` (Accepted) | Kept; dated note in Consequences |
| `ADR-54` (Accepted) | Kept; dated note in Consequences, pointing at `ADR-77`'s amendment |
| `RFC-23` (Accepted) | Kept; dated note section |
| `RFC-38` (Accepted) | Kept; dated note section |
| `RFC-26` (Draft) | Kept; dated note section |

## Done means

- Each place that relies on there being no external adopters (`AGENTS.md`'s "Don't build speculative capability" and the six other places the What lists, each checked by grep) is read, the ones that still hold are kept and the ones that do not are rewritten, and the outcome is recorded. — evidence: code:AGENTS.md@sha256:4260777828689cf9fdfc871943e6dc4b58b66b53053087d23da1b01b1ac2962e, code:rust/crates/urzua-core/src/config.rs@sha256:921027c81b7ecf7d56c357dd548fa28c47c785c7cd322c261c14e0b46626b296, spec:docs/adr/ADR-44-pointer-and-narrative-fields-are-config-declared-per-type-not-hardcoded.md, spec:docs/adr/ADR-54-a-break-is-a-break-no-migration-diagnostics-for-formats-we-no-longer-read.md, spec:docs/rfc/RFC-23-config-declared-pointer-and-blocking-fields-instead-of-hardcoded-in-urzua-core.md, spec:docs/rfc/RFC-26-records-not-github-issues-close-the-issue-tracker-as-a-tracking-surface.md, spec:docs/rfc/RFC-38-a-read-only-subset-does-not-earn-a-command-fold-audit-into-check-and-stop-re-deriving-fix-s-detection.md
- An Accepted record is left alone and corrected by a dated note. — evidence: spec:docs/adr/ADR-44-pointer-and-narrative-fields-are-config-declared-per-type-not-hardcoded.md, spec:docs/adr/ADR-54-a-break-is-a-break-no-migration-diagnostics-for-formats-we-no-longer-read.md, spec:docs/rfc/RFC-23-config-declared-pointer-and-blocking-fields-instead-of-hardcoded-in-urzua-core.md
- A grep for the premise's wording lists only places with an outcome. — waived: process-outcome -- a search result, not a file; at the time of this migration the wording appears only in the seven noted places and in the records that cite the decided expiry (ADR-77, RFC-49)

## References

- `MILE-102` -- replace a hand-written linter outright; the first adopter this premise predates.
- `SPEC-19` -- governs `AGENTS.md`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** A first real adopter corpus is now the Phase 0 target while seven places still justify decisions by there being none; filed to re-read each, leaving Accepted records to dated notes. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 0 to Phase 1; `Status` unchanged. Not needed to replace the adopter's linter. **Why:** maintainer decision of 2026-11-04 setting the Phase 0 exit (urzua governs its own repo in CI with zero standing warnings, and replaces one adopter's hand-written linter, which is then deleted); the exit is the adopter's linter deleted and urzua's own repo clean in CI, and this work is on neither path, so it moves out of Phase 0. | **substantive** |
> | 2026-11-04 | Re-phased from Phase 1 to Phase 0 (pulled into Phase 0 as foundation work); `Status` unchanged. **Why:** Maintainer decision of 2026-11-04 defining Phase 0 as parity on a solid codebase (`SPEC-1`'s Phase 0 exit): it decides whether config and output changes may break callers, which gates every refactor in the foundation set, and adoption follows Phase 0. | **substantive** |
> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-05 | `Status` changed from `Planned` to `Done`; `Realized-by` added; Outcome section added. The premise holds through Phase 0 and expires at the Phase 1 switchover (`MILE-102`), when `RFC-49`'s compatibility contract takes over; each of the seven places carries a dated note and no Accepted decision text was edited. **Why:** Maintainer decision, 2026-11-05; the PR that delivers the notes sets the evidence and `Done` together. Note: `RFC-38` is now Accepted, so it took a dated note like the other Accepted records, not an in-place edit. | **substantive** |
> | 2026-11-07 | Done means rewritten into ADR-96 items with per-item evidence (MILE-215). | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 2 locators pinned, 2 file-level, 0 scoped. Evidence: AGENTS.md@sha256:0d60e75506df, rust/crates/urzua-core/src/config.rs@sha256:18818ee3b226 Reason: Pinned the files as they are now after checking each against the record it evidences: report.rs still discloses every non-absent exclusion in a rule's population (MILE-111), catalog.rs still registers header.case-variant-key (MILE-113), export.rs still emits each record as plain JSON and generate-dashboard.py still reads it (MILE-115), the Makefile and checks.yml still run the zero-warning gate (MILE-116), AGENTS.md and config.rs still state and read the declared-adopter premise (MILE-118), diff_scope.rs still selects the records a --base run examines (MILE-123), boundary.rs, back_pointer.rs and the corpus-h expectation still implement and prove the declared published-path boundary (MILE-125), the claim_expiry.rs, reverify.rs and commands/reverify.rs files and the changeset still implement ADR-97 beside embodiment.rs, discovery.rs and fix.rs (MILE-129), discovery.rs, new.rs and the corpus-i files still give one identity namespace across directories (MILE-130), and the rules.rs module index and the per-family rule modules still exist as the split MILE-131 describes. **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `AGENTS.md` 0d60e75506df -> 426077782868. Reason: AGENTS.md changed only in its Accepted-risk list (commits seen on the checked-out branch, MCP shell and patch tools, invalid configuration, the unprotected instructions block, ungated waivers, which repositories judge an edit); the sections this record cites are unchanged. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 18818ee3b226 -> df8b21e9c8a5. Reason: split parse_with_document_models, validate_relation_pairs, claim_expiry_problems and revision_log_problem in config.rs into named validation helpers (SPEC-22 100-line bound); behaviour unchanged, outputs byte-identical, validation order and messages pinned by the new validation_order_tests **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` df8b21e9c8a5 -> 20203e601224. Reason: Read the diff of each pinned file or range since the old pin against what this record evidences: config.rs gained the validate_ignore_markers helper, called first in validate_boundaries, which accepts ignore_marker on embodiment.back-pointer-resolution and embodiment.back-pointer-decision-accepted, validates the marker (non-blank, one line, no padding) ahead of the boundary-only checks so it covers every rule that takes the option (this replaces the boundary rule's inline check, with the same errors), refuses a marker inside the Implements: or Verifies: keyword on the two back-pointer rules and refuses two different markers on them; no other option, parse or default changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and a reviewer re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/config.rs` 20203e601224 -> 921027c81b7e. Reason: config.rs: read the diff since the previous pin; it adds generic_marker_problem and its call in validate_ignore_markers (a back-pointer ignore_marker shorter than 3 characters, with no letter or digit, or inside a record id head is refused at load) plus its test; nothing this record's claim covers changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
