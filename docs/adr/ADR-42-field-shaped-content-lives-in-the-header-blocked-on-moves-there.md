---
Status: Accepted
Stable-Id: 01M1ZAERYDAF7ANT3GPFAH2HFW
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale@sha256:2d023f71c18daba3d6f77fcb0ed04238f651bee0a4b232212dac3aa12db5e1b6, code:.urzua/templates/milestone.md@sha256:878708705fa067c5365be3434ce2ab904f0b29b72615df3595c8856e01b3597e, code:.urzua/config.yaml#record_types::milestone@sha256:09c2c805d84b55a38249ccb6dbcc959aaa089339920a28920db640c7d87da325, code:.urzua/config.yaml#rules::narrative-field.stale@sha256:a43d98bc437f7e71b9b556b74b47ce757a3ef0f4735943873833df223919f858, code:scripts/generate-dashboard.py@sha256:1fcbc09112b61edca0f8c721fda79412ed2b1541a4e41e4ce777b18d1d3685ab, test:rust/crates/urzua-core/src/rules/narrative.rs#a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding@sha256:6a29cc6cc16f6aa9b5b2ac71337ca9514d6e800f1ee8593067ed50ba0fb72265
Date: 2026-09-07
Accepted-on: 2026-09-07
Author: beauwilliams
Deciders: beauwilliams
Supersedes / Superseded-by: —
Derives-from: ADR-40
---
# 42 — Field-shaped content lives in the header; Blocked-on moves there

## Context

Backlog triage found that MILE-2 and MILE-3 both listed `## Blocked on: Milestone: Fix urzua new's
template-priority bug` — prose describing a bug (BUG-3) that had already shipped (`Status: Fixed`).
Nothing caught this: `Blocked on` lived in a body section, not a checked header field, so a blocker
could silently resolve and nobody would notice the record was never revisited.

The first design draft kept `Blocked on` as a body section and proposed a new section-parser to
check it, mirroring `revision_log_change_class`'s pattern of reading a specific body section by
marker. This was pushed back on directly: every other reference field in this schema (`Implements`,
`Derives-from`, `Parent`, `Supersedes / Superseded-by`) already lives in the header. Checked against
all six templates (`.urzua/templates/*.md`) before committing to a broader claim: every other body
section across ADR/RFC/bug/milestone/waiver (`Context`, `Decision`, `What was wrong`, `Why`,
`Summary`, `Proposal`, `What this covers`, etc.) is genuine narrative prose — `Blocked on` was the
**one and only** field-shaped exception in the entire schema.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Keep `Blocked on` as a body section; add a new section-parser to check it | No corpus migration | A second, separate reference-checking code path alongside the header-field one; inconsistent with every other reference field |
| Move `Blocked on` into the header; migrate the corpus | Consistent with `Implements`/`Derives-from`/`Parent`; reuses `pointer_resolution` directly, no new parsing code | Requires migrating all 84 milestone files (mechanical, scriptable, not free) |

## Decision

In the context of one field-shaped exception sitting outside an otherwise-consistent schema, facing
a choice between building a second parsing path or migrating the one outlier, we decided: **field-
shaped content lives in the header; only narrative prose lives in body sections.** `Blocked-on`
moves into `milestone`'s header (renamed with a hyphen, matching `Found-in`/`Derives-from`'s
convention), alongside `Implements`. All 84 milestone files — including already-`Done` ones, to
avoid a half-migrated corpus — were migrated by a one-off script, verified by a before/after content-
equality check (whitespace-normalized) rather than spot-checked by eye; the one deliberate exception
(MILE-81, whose collapsed value described a *different* record's blocker and no longer made sense
verbatim once MILE-81 itself was `Done`) was hand-fixed and folded into its own `## Why` section
instead of discarded.

Two rules now cover it, reusing existing mechanisms rather than adding new ones:
- **`pointer_resolution`** (ADR-40) gained `"Blocked-on"` in its scanned-field list — a dangling
  reference is its existing error case, reused for free. Free text with no reference token (`"a
  decision not yet made"`) stays legal; `extract_references` finds nothing to check.
- **A new rule, `blocked_on_stale`**: for a `Blocked-on` reference that resolves, checks the
  target's `Status` against a small, hardcoded per-type terminal-status set (`bug` →
  `Fixed`/`WontFix`; `adr`/`rfc` → `Accepted`/`Rejected`/`Superseded`; `spec` → `Accepted`;
  `milestone` → `Done`/`WontDo`) — MVP, not config-driven, matching ADR-18's own "ship the MVP,
  extend once a real case demands more" precedent. `Warning` severity: a signal to re-examine, not
  a hard error.

Verified against the real, live case rather than only synthetic fixtures: MILE-2/3's migrated
`Blocked-on: BUG-3` was left unedited immediately after migration, `blocked_on_stale` was confirmed
to fire on it, and only then was the actual fix (clearing the blocker) applied.

## Reversibility

The rule itself is cheap to remove (one function, one array entry). The corpus migration is the
harder-to-reverse part — reverting would mean re-migrating 84 files back to body sections — but
nothing about record identity, numbering, or cross-references changed, only where one field's value
lives physically in the file.

## Consequences

- `scripts/generate-dashboard.py` updated (`section(content, "Blocked on")` → `field(content,
  "Blocked-on")`) — a real, live regression this decision would otherwise have shipped silently
  into the committed dashboard generator (ADR-37).
- `.urzua/templates/milestone.md` and `known_fields` for `milestone` (ADR-39) both updated so new
  milestones and `header.field-set-consistency` agree on the new field.
- A dangling `Blocked-on` reference is deliberately not `blocked_on_stale`'s job — that duplication
  was rejected in favor of reusing `pointer_resolution`'s existing error case.
- MILE-18 and MILE-40, found stale by the same triage that surfaced MILE-2/3, were corrected
  alongside this work (identity-resolution rescoping; dropping a hardcoded changeset count from a
  permanent title) — not because this rule caught them (neither cites a record ID in `Blocked-on`
  today), but because they were found by the same review pass.

## Amendment (2026-11-04): the stale-blocker rule is config-driven

*Status of this amendment: accepted 2026-11-04 by the maintainer (drafted 2026-11-04 from the drift review the maintainer approved). The Decision text above is unchanged.*

The Decision describes a rule `blocked_on_stale` with "a small, hardcoded per-type terminal-status
set", at `Warning` severity, "MVP, not config-driven". That rule no longer exists under that name.
`ADR-44` made it `narrative-field.stale` (`rust/crates/urzua-core/src/rules/narrative.rs`): the
narrative fields it watches are declared per type, and the statuses that count as terminal are the
rule's `terminal_statuses` option in `.urzua/config.yaml`, with severity set there (`warn`). The
placement decision stands: `Blocked-on` lives in `milestone`'s header beside `Implements`, and a
dangling reference is still `pointer.resolution`'s finding, not this rule's.

## References

- ADR-40 — the `Parent`-pointer precedent this decision extends `pointer_resolution` with again.
- ADR-18 — the "ship the MVP, extend once a real case demands more" precedent for the hardcoded
  terminal-status set.
- ADR-32 — `embodiment.consistency`'s drift-detection precedent, the same *shape* of problem
  (a claim staying trustworthy only if a rule watches it) this rule solves for `Blocked-on`.
- ADR-37 — the dashboard generator this decision's migration required updating.
- MILE-83 — the milestone this ADR resolves.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | `Realized-by` locator updated from `.urzua/config.toml` to `.urzua/config.yaml`. **Why:** `ADR-52` renamed the file on 2026-09-17 and this record kept naming the old path, so its embodiment claim pointed at nothing for two days. Found by `embodiment.locator-exists`, the rule written because nothing had noticed (`BUG-49`). | **substantive** |
> | 2026-11-04 | `Realized-by` re-pointed at where the evidence now lives, each locator checked against the code it names. **Why:** the locators named files that have since been split or reorganised (`rules.rs` into `rules/<family>.rs`, `main.rs` into `commands/<command>.rs`), so the recorded `Embodiment` could no longer be re-verified; the Decision is unchanged. | **substantive** |
> | 2026-11-04 | `Embodiment` set to `Drift detected`. **Why:** Re-verified against current code; maintainer decision 2026-11-04. The Decision describes a hardcoded `blocked_on_stale` rule with a fixed terminal-status set; the code now has the config-driven `narrative-field.stale` (`ADR-44`). | **substantive** |
> | 2026-11-04 | Backfilled `Accepted-on` from git history, commit cf6e4da33. | **structural** |
> | 2026-11-04 | Amendment added: `blocked_on_stale` and its hardcoded terminal-status set became the config-driven `narrative-field.stale` (`ADR-44`). **Why:** drift review of 2026-11-04, approved by beauwilliams; proposed, pending maintainer acceptance. The Decision's header-placement ruling is unchanged. | **substantive** |
> | 2026-11-04 | Pinned `rust/crates/urzua-core/src/rules/narrative.rs`, `.urzua/templates/milestone.md`, `.urzua/config.yaml`, `scripts/generate-dashboard.py` by content hash after re-verifying it against the Decision (attested by beauwilliams). **Why:** Re-pinned with the amendment above: the header-placement decision holds and the rule change is recorded. | **structural** |
> | 2026-11-04 | Tool-authored (by beauwilliams): recomputed `Embodiment` from `Realized-by: code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:9fff38b1c9f672afd6c4c90788b14d177a9c12cebec643448a344e1291c572b6, code:.urzua/templates/milestone.md@sha256:0abec9285fd32135b348a4646a24a99173689397f6863186c863314087d26bc5, code:.urzua/config.yaml@sha256:8bc61e85bdcce1d754a31cf5b9d304cd0df564d848a7fb50cc263e82b7fe5d46, code:scripts/generate-dashboard.py@sha256:b163ade4df40293fc53b8eb0f8d045d6c0d5d1b125c1d42012f30de782ef07bb, test:rust/crates/urzua-core/src/rules/narrative.rs@sha256:9fff38b1c9f672afd6c4c90788b14d177a9c12cebec643448a344e1291c572b6` -- stated `Drift detected` disagreed with the computed value `Verified`. | **structural** |
> | 2026-11-04 | Amendment of 2026-11-04 accepted. **Why:** Maintainer decision, 2026-11-04; the Decision text is unchanged. | **substantive** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `.urzua/templates/milestone.md` 0abec9285fd3 -> a3a9121a71ba, `scripts/generate-dashboard.py` b163ade4df40 -> 1c64e16664a6. Reason: Re-verified against the Decision and its accepted amendment of 2026-11-04. The remaining file-level pins changed as follows: milestone.md gained a Done means section and a descriptive Phase placeholder; generate-dashboard.py reads declared sections from urzua export instead of raw text. Neither touches the ruling that field-shaped content (Blocked-on) lives in the header, which the milestone type in config.yaml still declares as a known and narrative field. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Narrowed the evidence pins on volatile files to scoped pins (`ADR-74`, `MILE-185`): `.urzua/config.yaml` to `record_types::milestone` and `rules::narrative-field.stale`, `rules/narrative.rs` to `narrative_field_stale` (code) and `a_blocked_on_pointer_to_a_terminal_bug_is_a_stale_finding` (test). Reason: drift review of 2026-11-04. The milestone type still declares `Blocked-on` as a header field (`known_fields`, `narrative_fields`), and `narrative-field.stale` reads its terminal statuses from config, as the accepted amendment records. Unrelated edits to those files no longer make this record stale. **Why:** the pins were set on whole files that change for reasons unrelated to this decision. | **structural** |
> | 2026-11-05 | Re-baselined (BUG-50, attested by this PR's review): re-pinned `.urzua/config.yaml#record_types::milestone` 9ad312f01e5d -> 34ac511e6e4a. Reason: BUG-50 added one `revision_log: {marker, required: true}` declaration to the milestone type; the header-field declarations this record cites are unchanged **Why:** the pinned evidence changed and the PR review is the human re-verification that the claim still holds. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `.urzua/templates/milestone.md` a3a9121a71ba -> dc49935b1c0f, `.urzua/config.yaml#record_types::milestone` (lines 213-276) 34ac511e6e4a -> 10cf76424dcd, `scripts/generate-dashboard.py` 1c64e16664a6 -> 1fcbc09112b6. Reason: Re-verified against the Decision (field-shaped content lives in the header; Blocked-on is a declared narrative field): the milestone type's declaration changed only by dropping Blocked from its Status vocabulary and declaring progress, and the milestone template and dashboard status lists dropped Blocked. It still declares Blocked-on a known and narrative header field. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-05 | Corrects the row of 2026-11-05 that re-pinned `.urzua/templates/milestone.md`, `.urzua/config.yaml#record_types::milestone` (lines 213-276) 34ac511e6e4a -> 10cf76424dcd and `scripts/generate-dashboard.py` (`#313`). Dropping `Blocked` from the milestone `Status` vocabulary also changed `initial` (`Blocked` removed), `transitions` (the `Blocked` row and every `Blocked` target removed) and `delivery.not_done` (`Blocked` removed), and added `progress`; the earlier reason named only the vocabulary and `progress`. The milestone type still declares `Blocked-on` a known and narrative header field. **Why:** an independent check of the re-pin reasons found the earlier row incomplete or inaccurate; corrected here without rewriting it. | **structural** |
> | 2026-11-05 | Re-baselined (attested by beauwilliams): re-pinned `.urzua/templates/milestone.md` dc49935b1c0f -> 8d488d4a2a32, `.urzua/config.yaml#record_types::milestone` (lines 242-306) 10cf76424dcd -> afb9458a285e, `.urzua/config.yaml#rules::narrative-field.stale` (lines 570-570) ca554926d42c -> a43d98bc437f. Reason: milestone.md template lost its optional Implements line (Blocked-on stays in the header); the milestone type gained the Non-reference-ids known field; narrative-field.stale lost its inert terminal_statuses list, now that every status-bearing type declares success and failure. The rule is still enabled at warn and narrative.rs is unchanged. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-06 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/templates/milestone.md` 8d488d4a2a32 -> bebac786c6a9, `.urzua/config.yaml#record_types::milestone` (lines 242-306) afb9458a285e -> fdd20f805e74. Reason: The milestone record type in .urzua/config.yaml now lists Stable-Id under required_fields (moved from known_fields), and .urzua/templates/milestone.md gained a Stable-Id header line; the Blocked-on header-field declarations this decision cites are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale` (lines 3-95) 5a42295a56e6 -> 9ff736315eee. Reason: Doc comment of narrative_field_stale rewritten to say terminal statuses are declared, not hardcoded; no behaviour change **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: unspecified): re-pinned `.urzua/config.yaml#record_types::milestone` (lines 280-350) fdd20f805e74 -> 327ddbea72e2. Reason: The milestone type's revision_log declaration in .urzua/config.yaml now also declares classes, class_column, date_column and tool_class with today's values (MILE-213, ADR-94); the type's other keys are unchanged and behaviour is identical. **Why:** the pinned evidence changed and the attesting actor re-verified that the claim still holds. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::milestone` (lines 280-365) 327ddbea72e2 -> 133922fe07cf. Reason: The milestone record type in .urzua/config.yaml gained statuses.delivery.items (the ADR-96 Done-means item declaration, MILE-215), on top of the revision-log class keys MILE-213 added; the rest of the pinned span, including the field-shaped and Blocked-on declarations this decision cites, is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::milestone` (lines 280-378) 133922fe07cf -> 770f07d86162. Reason: The pinned record_types::milestone block of .urzua/config.yaml gained the Kind header field in known_fields and a statuses.delivery.items.kind declaration (build and decision kinds). The Blocked-on narrative_fields declaration this record cites is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::milestone` (lines 283-369) 133922fe07cf -> 8f469d82002a. Reason: Read the diff of each pinned span since the old pin and the claim still holds. config.yaml#record_types::milestone: this change added Last-audited to the type's known_fields, the rest of the declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::milestone` (lines 283-382) 8f469d82002a -> 8921d05bfe75. Reason: Read the diff of each pinned file since the pin taken earlier in this branch (the changes main landed in ADR-98's sharded CI and the milestone Kind amendment) and the claim still holds. config.yaml#record_types::milestone: the milestone type's span gained Kind among its known fields and a kind declaration (build and decision) under statuses.delivery.items (ADR-96 amendment of 2026-11-07), and the rest of the declaration is unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/templates/milestone.md` bebac786c6a9 -> 878708705fa0. Reason: Read the diff of each re-pinned file against main. templates/milestone.md: Done means is scaffolded as placeholder list items instead of prose guidance; header and other sections unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-08 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `.urzua/config.yaml#record_types::milestone` (lines 283-388) 8921d05bfe75 -> 09c2c805d84b. Reason: Read the diff of each pinned file since the old pin and the claim still holds. config.yaml#record_types::milestone: the only change is a new pending list (two placeholder patterns) at the end of statuses.delivery.items; the type's fields, header shape and Blocked-on handling are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
> | 2026-11-09 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/rules/narrative.rs#narrative_field_stale` (lines 3-86) 9ff736315eee -> 2d023f71c18d. Reason: Re-pinned after checking the delta in each pinned file against what this record evidences: narrative.rs: Finding/RuleExecution literals replaced by report.rs constructors; field values unchanged. Behaviour unchanged, outputs byte-identical (check, check --base, audit, fix --rebaseline, reverify, rules and explain --rule JSON compared before and after on this repo and five acceptance corpora, with and without --as-of). **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
