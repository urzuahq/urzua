---
Status: Accepted
Stable-Id: 01M1ZAERYDAF7ANT3GPFAH2HFW
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/rules/narrative.rs@sha256:9fff38b1c9f672afd6c4c90788b14d177a9c12cebec643448a344e1291c572b6, code:.urzua/templates/milestone.md@sha256:0abec9285fd32135b348a4646a24a99173689397f6863186c863314087d26bc5, code:.urzua/config.yaml@sha256:8bc61e85bdcce1d754a31cf5b9d304cd0df564d848a7fb50cc263e82b7fe5d46, code:scripts/generate-dashboard.py@sha256:b163ade4df40293fc53b8eb0f8d045d6c0d5d1b125c1d42012f30de782ef07bb, test:rust/crates/urzua-core/src/rules/narrative.rs@sha256:9fff38b1c9f672afd6c4c90788b14d177a9c12cebec643448a344e1291c572b6
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
