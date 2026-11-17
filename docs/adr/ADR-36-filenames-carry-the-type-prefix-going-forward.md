---
Stable-Id: 01M6R3NK4Q2YZJPBKACXX0VD68
Status: Accepted
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/filename.rs@sha256:737789b4240bb8282637dd48e08d66d6cebc47786a829fc2ac6fc4ba42d17207, code:rust/crates/urzua-core/src/new_record.rs@sha256:08fe0a8a09c65fa0b984fd646a0b015fd654e78254d0abfe85bbbe58788914b4, test:rust/crates/urzua-core/src/rules/filename.rs@sha256:737789b4240bb8282637dd48e08d66d6cebc47786a829fc2ac6fc4ba42d17207, test:rust/crates/urzua-core/src/new_record.rs@sha256:08fe0a8a09c65fa0b984fd646a0b015fd654e78254d0abfe85bbbe58788914b4
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-3, ADR-34
Non-reference-ids: ADR-9999, ADR-10000
---
# 36 — Filenames carry the type prefix going forward

## Context

BUG-2 already removed the actual defect in numbering (any digit-length filename prefix resolves,
matching is by numeric value not exact string) — what remained was a considered style question,
raised directly: should a record's filename carry its type prefix explicitly
(`ADR-36-slug.md`), rather than relying on directory context the way `docs/adr/0036-slug.md`
does today? And should zero-padding and the human-readable slug stay, now that neither is required
for correctness?

## Decision

In the context of a filename convention question with no remaining correctness defect behind it,
we decided on three points together:

- **Type prefix moves into the filename**: `TYPE-NNNN-slug.md` is the default `urzua new` emits
  going forward. This matches how a record is already referenced everywhere else in this corpus —
  `Implements: ADR-36` is the same string as the file's own name, not a translation of it.
- **Zero-padding stays**, despite no longer being required for parsing. A plain `ls`, `git log`, or
  GitHub's file browser all sort lexicographically, not numerically — padding is what keeps that
  sort order matching creation order once past 9 or 99 records of one type.
- **The slug stays.** Genuinely useful for at-a-glance readability in a file listing without
  opening the file, and for any future generated web view. Staleness against the title is already
  a solved problem (`filename.title-consistency`), not a new cost this decision introduces.
- **Legacy `NNNN-slug.md` filenames are never renamed and never stop working** — the same
  non-disruptive precedent ADR-33 already set for header shapes: parsing accepts both forms
  permanently, `record_id`/`filename_number` try the type-prefixed form first, then fall back to
  the legacy form. This repo's own existing 95 records are untouched by this decision.

A second, independently-found instance of BUG-2's exact defect class was fixed in the same
pass: `filename_title_consistency`'s own `filename_number`/`title_number` helpers had their own
separate hardcoded 4-digit checks, missed when `record_id` was fixed. Both now accept any digit
length and compare by numeric value, matching `record_id`/`normalize_id`'s already-established
approach.

## Reversibility

Additive and non-disruptive: existing filenames are untouched, both shapes parse identically
forever. Only what `urzua new` emits for brand-new records changes.

## Consequences

- `docs/adr/`, `docs/rfc/`, etc. will hold a mix of legacy and type-prefixed filenames indefinitely
  — this is expected, not a migration-in-progress state to eventually finish.
- `next_display_number` reads both shapes to find the true max across a mixed directory, so
  numbering never collides regardless of which shape any individual file happens to use.
- A mass rename of this repo's own 95 existing records to the new shape is explicitly not decided
  here — possible later, purely cosmetic, never required.

## Amendment (2026-09-07): zero-padding reversed

The padding half of this decision is reversed. Re-examined directly: fixed-width padding does not
solve the lexicographic-sort problem it was chosen for, it only defers the point where it breaks —
once any one type crosses the padded width (e.g. a 5-digit `ADR-10000` next to four-digit
`ADR-1`..`ADR-9999`), the directory is *still* lexicographically out of order, and now with
mixed-width names, which is a worse break than the one padding was meant to prevent. There is no
width that makes this permanent; every fixed width just picks when the break happens.

Unpadded numbers don't solve lexicographic sort either, but they never claimed to — a plain `ls`
has never sorted `ADR-2` before `ADR-10`, padded or not, past two digits without leading zeros
matching. Padding's actual value was cosmetic consistency at small scale, at the cost of an
eventual, harder break. Not worth it.

Decision, superseding the padding bullet above: **no zero-padding, anywhere** — filenames
(`ADR-36-slug.md`, not the four-digit `ADR-0036-slug.md`) and each record's own H1 heading both show the bare
number. The type-prefix-in-filename, slug-stays, and legacy-names-stay-valid-forever bullets are
unchanged. `urzua new` emits unpadded numbers as of this amendment; existing records are renamed
separately, in one pass across the whole corpus, tracked as its own piece of work rather than
folded silently into this note.

## Amendment (2026-09-09): legacy-filename acceptance removed

`BUG-9` found this Decision's "legacy names stay valid forever" bullet borrowed ADR-33's
non-disruptive-deprecation precedent onto a case that doesn't share its justification. ADR-33's
permanent parsing for `blockquote`/`bold-list` headers has a real, ongoing reason: any future
adopter's own pre-existing corpus will legitimately use those shapes on day one, before they've
decided anything about this tool. The legacy `NNNN-slug.md` filename shape has no equivalent case —
no one arriving at their own corpus for the first time chooses a bare-number filename over a typed
one; it only ever existed as this repo's own pre-ADR-36 history. That history no longer exists
either: checked live, zero files in this corpus use the legacy shape today, falsifying this
Decision's own Consequences claim ("will hold a mix ... indefinitely") from the moment it was
written, not just by drift since.

**Decision, superseding the fourth bullet above: legacy `NNNN-slug.md` filenames are no longer
accepted.** `record_id`, `filename_number`, and `next_display_number` now recognize only the
type-prefixed `TYPE-NNNN-slug.md` shape; a reference to a bare-number filename is dangling, the same
as a reference to any other nonexistent record. The type-prefix-in-filename, no-zero-padding, and
slug-stays bullets are unchanged.

This ADR's own H1 title is edited by this amendment, dropping the now-false "legacy names stay
valid forever" clause it asserted — the title is a live claim like any other, not exempt from the
correction this amendment makes to the Decision itself; the original wording is preserved above in
the unedited Decision/Consequences sections and this amendment's own text.

## References

- BUG-2 — the defect this decision's remaining scope is purely stylistic on top of.
- BUG-9 — the live audit (zero legacy-shaped files in the corpus) this amendment acts on.
- ADR-33 — the non-disruptive-deprecation precedent this decision originally, and mistakenly,
  extended to filenames; still correct for header shapes, which retain a real ongoing case this
  amendment does not touch.
- ADR-3 — display-number-is-identity, unchanged by this decision.
- `rust/crates/urzua-core/src/rules.rs` — `record_id`, `filename_number`, `title_number`.
- `rust/crates/urzua-core/src/new_record.rs` — `next_display_number`.
- `rust/crates/urzua-cli/src/main.rs` — `run_new`'s filename construction.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Realized-by` locators pinned by content hash (`@sha256:`) after re-verifying each cited file against the Decision. **Why:** Re-verified against current code; maintainer decision 2026-11-04. | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit 302571381. | **structural** |
> | 2026-11-04 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` d1dfa0bf3267 -> 6e014472fce7, `rust/crates/urzua-core/src/new_record.rs` 5473850e4d7d -> 15511d30876e. Reason: Re-verified: new_record.rs now matches the record's two amendments (width-padded names from the identity pattern, unpadded when open-ended). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` 6e014472fce7 -> fbf1f65873e3. Reason: Drift review of 2026-11-04, recommendation (a). The only change to filename.rs since the pin is an action_source added to three findings (ADR-73, #275); the identity-pattern and title checks are unchanged. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `rust/crates/urzua-core/src/new_record.rs` 15511d30876e -> 850e21078331. Reason: Drift review of 2026-11-04, recommendation (a), for a later change from main (BUG-22, #276). The only change to new_record.rs since the pin is added code: a TemplateValues type, template_header_values and a test, which let field.quality read a template's declared values. Nothing the filename-prefix convention, zero-padding or slug behaviour of urzua new depends on was removed or changed. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Added the `Non-reference-ids` header field declaring `ADR-9999, ADR-10000`. **Why:** the text names these ids as history or illustration, not as citations of a record; the declaration states that on the record itself, so a new unresolved citation in this file is reported, where a file-wide `citation.resolution` waiver had silenced every one (`BUG-190`, `ADR-75`). The field carries no decision; the Decision is unchanged. | **structural** |
> | 2026-11-07 | Corrects the 2026-11-04 row that re-pinned `rust/crates/urzua-core/src/rules/filename.rs` d1dfa0bf3267 -> 6e014472fce7, whose reason described only `new_record.rs`. The `filename.rs` delta (`MILE-140`, `MILE-85`): a `declared_severity: None` field added to the findings the file builds, a doc comment on `first_h1` reworded from blockquote and bold-list headers to a `prefix-lines` header, and the file's test fixtures moved from blockquote headers (`> Status: Accepted`) to YAML frontmatter. No change to the identity-pattern or title checks this record cites, so the claim still holds. **Why:** an independent check of the re-pin reasons found the earlier row incomplete; corrected here without rewriting it. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` fbf1f65873e3 -> 10e3d229281d, `rust/crates/urzua-core/src/new_record.rs` 850e21078331 -> 21f5012c2b5f. Reason: Read the diff of each pinned span since the old pin and the claim still holds. rules/filename.rs: title-id reading is stricter: h1_id reads PREFIX-N or a declared prefix capture and never turns a date or version into a number, an overflowing id is reported, and the rename list in a finding is capped at five names, the rule still compares the filename number with the H1 id; urzua-core/src/new_record.rs: next_display_number and the number assignment now return a Result and refuse an exhausted number instead of overflowing, new records may fill a declared title field and initial version, and a required Stable-Id is no longer overwritten with null, the filename conventions are as before. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` 10e3d229281d -> eb654501113d. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: filename.rs: split filename_identity_pattern into identity_findings_for_type. Behaviour unchanged, outputs byte-identical (check, check --base and fix --rebaseline JSON compared before and after). **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` eb654501113d -> b8eaa6832564. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: filename.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` b8eaa6832564 -> 78e015d40b70. Reason: Read the diff against the pin. filename.rs: expected_name strips the .md extension through record::strip_md_extension (any case of the extension) instead of strip_suffix(".md"); lowercase names behave as before. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 21f5012c2b5f -> 0f9f2d4f6a94. Reason: Read the diff of each pinned file since the previous pin. rust/crates/urzua-core/src/new_record.rs: parse_record_filename reads the extension through strip_md_extension (any case), with a unit test; nothing else changed. The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/new_record.rs` 0f9f2d4f6a94 -> 258765bfb791. Reason: Read the diff of each pinned file against main. rust/crates/urzua-core/src/new_record.rs: added ScaffoldSpec, scaffold_record and a private declared_template_fields (the header-and-body scaffold moved here from the CLI) plus one unit test; no existing function changed. Nothing else changed in the pinned file, and the output of urzua new is identical. The claim this record evidences still holds. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 258765bfb791 -> 40f0fce95cd0. Reason: Read the diff of each re-pinned file against main. new_record.rs: ScaffoldSpec gained optional Reason, Expires, header-pair, log-row and extra-body fields and a ScaffoldSpec::plain constructor; scaffold_record applies them only when set, and render_synthetic_yaml now delegates to a private render_yaml_with with no supplied values, so the output for urzua new is unchanged; added with_first_log_row and three unit tests (identity padding, supplied values, nothing supplied). The claim this record evidences still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 40f0fce95cd0 -> eab8490554bb. Reason: Re-pinned after PR4 of MILE-205 (urzua waive --apply), the stated delta being: new_record.rs: ScaffoldSpec gains omit_fields and the YAML renderer drops those fields; new passes none, so its output is unchanged. The claim holds as the changed code does not alter what this record states. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-13 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` 78e015d40b70 -> e3647c63a826, `rust/crates/urzua-core/src/new_record.rs` eab8490554bb -> 4f85670917b9. Reason: Read the diff of each re-pinned file against main and the claim still holds. new_record.rs: one doc comment names PREFIX-999 where it named ADR-999; no code changed; filename.rs: one doc comment names ADR-0017 and ADR-17 where it named ADR-0173 and ADR-173; no code changed. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 4f85670917b9 -> 203bcd8b4321. Reason: new_record.rs dates a template revision-log row whose date cell is YYYY-MM-DD with the day the record is made; the record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 203bcd8b4321 -> 9ad96b4bc331. Reason: new_record.rs makes the scaffold's reason a (field, value) pair so a declared waiver_fields.reason is written. The claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` 504a0bd930dc -> e8b463b66dfa. Reason: filename.rs: the exclude_files alternative in the rename guidance moves into its own helper, exclude_files_hint; the message text is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` e3647c63a826 -> 504a0bd930dc. Reason: filename.rs: filename.identity-pattern's rename guidance now names exclude_files as the alternative for a file with no number; which files it reports is unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-16 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/filename.rs` e8b463b66dfa -> 737789b4240b. Reason: filename.rs: rename guidance names exclude_files for a numberless file, and a file with no identity is counted with the no-identity outcome; findings unchanged **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 9ad96b4bc331 -> 048a39513924. Reason: new_record.rs writes a field new has no value for as the template's text, the em-dash sentinel when required, or leaves it out, never null, and returns the fields still to fill through scaffold_record_reporting; ScaffoldSpec gains unset_as_null for the comparison scaffold; the tests that pinned null now assert the new forms. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 048a39513924 -> 79c4d91d065f. Reason: new_record.rs writes a field new has no value for empty (Field:), never the template's text, the em-dash sentinel or null, keeps the same field set, and returns to_fill entries {field, required, hint} with the template's text as hint; the unset_as_null comparison flag is removed since an empty field reads as null; its unit tests assert the empty form. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 79c4d91d065f -> 5909f7fd05b3. Reason: new_record.rs starts a record at statuses.initial when declared (BUG-199), adds `allowed` (the type's field_values) to each to_fill entry and words the hint as that list when declared, offers no em-dash template value as a hint, with unit tests for each. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs` 5909f7fd05b3 -> 08fe0a8a09c6. Reason: new_record.rs adds StatusFrom and status_from(), which picks the first of statuses.initial (else the first of statuses.values) and lists the other initial states; the scaffold takes its starting status from it. The record's claim still holds. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **evidence** |
> | 2026-11-17 | The rows of 2026-11-17 (re-baseline) describe intermediate states of the pull request for `MILE-322` and `MILE-324` that the final change does not contain: a `ScaffoldSpec.unset_as_null` flag; writing a field `new` has no value for as the template's text, the `—` sentinel or nothing. The final change writes such a field empty and lists it in `to_fill`, has no such flag or function, leaves `is_record_reference`'s visibility as on main, and keeps a present but empty `Expires` expired, as releases 0.4.7 and 0.4.8 do. The earlier rows stay as history. **Why:** review of that pull request asked that rows naming designs absent from the final tree be closed. | **structural** |
