---
Status: Accepted
Stable-Id: 01M1Z17D93V6R1JMM21ETKZKV8
Embodiment: Verified
Realized-by: code:rust/crates/urzua-core/src/header.rs, code:rust/crates/urzua-core/src/config.rs, code:rust/crates/urzua-core/src/rules.rs, test:rust/crates/urzua-core/src/rules.rs
Date: 2026-09-07
Author: '@beauwilliams'
Deciders: '@beauwilliams'
Supersedes / Superseded-by: —
Derives-from: RFC-10 (Accepted)
---
# 38 — Header layout is declared per type, not templated or voted

## Context

`HeaderShape::Blockquote` deliberately tolerates several sub-formats interchangeably for parsing
(RFC-10): one field per line, bold-labelled fields, and several fields pipe-delimited on one line.
That tolerance is correct for *reading* a corpus that hasn't settled on one — but it also meant
`check` had no way to say a type's own records had drifted from each other, even once the type
plainly had settled: SPEC-2 through SPEC-6 all render pipe-delimited, SPEC-1 alone rendered
one-field-per-line, and nothing flagged it (MILE-75). Deciding whether this deserved a rule, and
what that rule should check against, is this decision.

Two shapes for "what's correct" were considered and rejected before this one. A rule could infer
the expected layout from majority vote among a type's existing records — rejected outright, the
same reasoning `header.required-fields` already uses for its own shape: "a majority-rule inference
would silently ratify whatever drifted in." A rule could instead derive the expected layout from
the type's own checked-in template (`.urzua/templates/<type>.md`), reusing ADR-27's "the template
is what `urzua new` fills in" precedent — appealing, since it adds no new config surface, but it
silently fails for exactly the corpus case that motivated this: `spec` has no template yet (MILE-74)
and never got a chance to fire on the SPEC-1 drift that exists today.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Majority-vote inference among existing records | No new config, works immediately | Ratifies whatever's already drifted in rather than catching it (rejected for Rule 1, for the same reason) |
| Derive from the type's checked-in template | No new config; reuses ADR-27's template-is-truth precedent | Silently inert for any type without a template yet — the exact case (`spec`, pre-MILE-74) this rule needed to catch |
| Declare `header_layout` per type in `.urzua/config.toml` | Works with or without a template; explicit, diffable, matches `header_shape`'s existing precedent exactly | One more optional config key to document |

## Decision

In the context of `HeaderShape::Blockquote` tolerating multiple sub-formats for parsing while a
type can still have visibly settled on one, facing a choice between inferring the expected layout
and declaring it, we decided: **`header_layout` is an optional per-type config field
(`"one-per-line"` or `"pipe-delimited"`), checked by a new `header.layout-consistency` rule.** A
type with no declared `header_layout` is skipped entirely — additive, never a forced migration.
`Header` gains a `layout()` method computed purely from already-parsed field line numbers (pipe-
delimited if two or more fields share a line, one-per-line otherwise) — no new parsing, no raw-text
re-scanning. Bold-vs-plain labelling, the other sub-format axis RFC-10 documents, is out of scope
for this MVP; nothing has needed it checked yet, and it ships only once a real case does, the same
staging discipline ADR-18 used for Embodiment.

This repo's own config declares `header_layout` for every type with more than one record today:
`adr`/`rfc`/`bug`/`milestone` as `one-per-line`, `spec` as `pipe-delimited` (matching SPEC-1's
normalization to SPEC-2 through SPEC-6's style, done alongside this).

## Reversibility

Fully additive and cheap to reverse: `header_layout` is an optional config key and `layout-
consistency` is one more rule in the existing list. Removing either leaves every other rule and the
existing schema untouched. Declaring it for a type that later needs to mix layouts on purpose is a
one-line config revert, not a schema migration.

## Consequences

- A new rule, `header.layout-consistency`, joins `check`'s existing rule set — `Warning` severity,
  consistent with how new rules have been introduced against a corpus not yet held to them.
- `RecordTypeConfig` gains `header_layout: Option<HeaderLayout>`; an unrecognized value is a config
  parse error, not silently ignored, matching `header_shape`'s existing deserialization.
- MILE-75's question is answered: sub-format drift within a type gets a rule, declared per type,
  not inferred. MILE-3 (converting this repo's docs to `yaml-frontmatter`) is independent of this —
  `YamlFrontmatter`-shaped types have no layout ambiguity to declare in the first place, so this
  rule simply stops having anything to check for a type once it migrates.
- Once `spec` gets a template (MILE-74), the template's own layout should match the declared
  `header_layout` — not enforced by this rule, since a template isn't itself a `Record` this rule
  examines, but worth a human eye whenever the template changes.

## Amendment (2026-09-08): header_layout removed once every type is yaml-frontmatter

This ADR's own Decision text stated `.urzua/config.toml` declares `header_layout` for `adr`/`rfc`/
`bug`/`milestone` (`one-per-line`) and `spec` (`pipe-delimited`) — accurate when written, and now
stale: ADR-33's amendment (the same date) widens the `yaml-frontmatter` migration to all six
configured types, and `header_layout`'s whole premise (distinguishing sub-formats *within*
`HeaderShape::Blockquote`) has nothing left to distinguish once no type declares `Blockquote` at
all. `header_layout` is removed from every type's config entry as part of that migration, and
`header.layout-consistency` returns to examining zero records for every type — not because the rule
broke, but because its declared-per-type axis is empty by construction, the same "additive, skip
when undeclared" behavior this ADR's own Decision already specified from the start.

The rule and the `HeaderLayout` enum stay in the codebase, unused rather than deleted: a future
type declaring `Blockquote` again (an external adopter's own config, not this repo's) would still
benefit from it, and removing working, tested code with no cost to keeping it is not this decision's
call to make.

## References

- RFC-10 — the closed-header model and the three tolerated sub-formats this decision narrows for
  types that have settled on one.
- ADR-18 — the "ship the MVP, extend only once a real case demands it" precedent this reuses for
  deferring bold-vs-plain labelling.
- ADR-27 — the template-is-truth precedent considered and not used here, since it can't cover a
  type without a template yet.
- ADR-33 — the amendment widening `yaml-frontmatter` migration to all six types, which is what
  empties this ADR's own declared axis.
- MILE-75 — the milestone this ADR resolves.
