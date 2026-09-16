---
Status: Draft
Date: 2026-09-07
Author: beauwilliams
Supersedes / Superseded-by: —
---
# 17 — Composable section-content checks: a closed section-parser, config-declared shapes, and a Y-statement checker

## Summary

`check` validates header fields today, but nothing validates *what's inside* a required section's
prose. Propose a shared, closed section-parser (mirroring RFC-10's closed-header model, one level
down), a config-declared `required_sections` list per record type where each section can carry an
optional content `shape`, and the first real shape — `y-statement` — which checks that a Decision
section's TL;DR follows the "In the context of X, facing Y, we decided Z, to achieve W, accepting
the downside V" structure. Along the way this resolves RFC-13 (a vacated identifier needs an
exemption from decision-shaped requirements) and closes a real, confirmed gap: nothing today checks
that a type's template agrees with its own config.

## Motivation

Three converging threads:

1. **Prose sections are currently unchecked.** `check` validates that required *header fields* are
   present and non-placeholder (`field.quality`), but a record's `## Decision` section can contain
   anything — an empty paragraph, a single sentence, or genuine reasoning — and nothing distinguishes
   them. SPEC-2 already names "required sections per profile" as a rule category; it has never
   been built.
2. **Whole-document YAML storage was considered and rejected as the fix.** The alternative explored
   first was moving prose sections into structured YAML values (one record, one YAML file, a
   generated markdown view). That doesn't actually make prose more checkable — a `decision: |` block
   scalar is exactly as unstructured as a `## Decision` heading, just with worse hand-editing and
   diff ergonomics for long-form text. The real fix is closing *where* a section is found, the same
   way RFC-10 closed *where* the header is found — not changing what format prose lives in.
3. **A live, confirmed drift risk with nothing checking it.** `urzua doctor` today checks that
   config exists, parses, that each type's directory exists, and that CI invokes `check` — and
   nothing else. It has zero awareness of `.urzua/templates/`. A type's template and its config can
   silently disagree (a required section renamed in config but not in the template; a `header_shape`
   changed without deleting the now-inconsistent template `urzua new` still prefers unconditionally
   — a related bug found live while exploring this design, tracked separately) with no mechanism
   catching it.

## Proposal

### 1. A closed section-parser, mirroring `header.rs`'s model

`header.rs`'s own stated principle — "parsed by exactly one function so no check runs its own regex
over raw text" — currently applies only to the header. Propose the same discipline for sections:
one function, `sections::parse(content: &str) -> Vec<Section>`, locating every `##`-level heading and
its body, tested once, closed. Every section-content rule operates on an already-extracted `&str`
(a section's body), never on raw document text — regex is fine *inside* a leaf-level content check
(does this already-extracted string contain phrase X), never for finding the boundary itself. A
naive per-rule regex risks matching a literal `## Decision` written as an example inside someone
else's fenced code block; one shared, structure-aware parser doesn't.

### 2. `required_sections`, declared in config, not inferred from the template

```toml
[[record_types.adr.required_sections]]
name = "Context"

[[record_types.adr.required_sections]]
name = "Decision"
shape = "y-statement"

[[record_types.adr.required_sections]]
name = "Consequences"

[[record_types.spec.required_sections]]
name = "Decision"          # no `shape` -- spec doesn't require the Y-statement structure
```

Config is the single source of truth for what a record type requires — matching how `required_fields`
already works, and matching SPEC-2's own stated principle ("rules are data where possible, not
code"). The template file stays a plain, hand-curated example with real authoring guidance text (the
existing templates' prompt sentences under each heading) — it is not parsed to *infer* what's
required. Two designs were compared for keeping template and config in agreement (§4 covers the
one adopted): fully generating the template from config was rejected because it would either lose
the hand-curated prompt text or duplicate it into config as a second copy to maintain — the same
drift risk relocated, not removed.

### 3. `y-statement`: the first content shape, and its correctness requirements

A content `shape` is a pure function: `(section_body: &str) -> Vec<Finding>`. `y-statement` is the
first one, checking a Decision section's TL;DR for five structural phrases ("In the context of",
"facing", "we decided", "to achieve", "accepting"). Three correctness properties, each grounded in a
concrete failure mode rather than asserted:

- **Extract the TL;DR block first; never scan the whole section for the phrases.** A section that
  mentions "we decided" anywhere outside the actual TL;DR line must not silently pass. Extraction
  uses a *lazy* match up to the next paragraph/list boundary — a greedy match would run to the last
  such boundary in the section, not the nearest one.
- **A marker found with no extractable block is its own distinct error** ("marker present but not
  followed by a recognizable boundary"), never a silent fallback to whole-section scanning — the
  fallback is exactly what would let the first bullet's failure mode back in.
- **Word-boundary phrase matching, not substring.** A bare substring search on "facing" matches
  "interfacing" or "surfacing" — a real, non-hypothetical false-positive class independently found
  and fixed twice in comparable prior art, mechanically avoidable by anchoring the match to word
  boundaries.

Shapes are meant to be composable and extensible — `y-statement` is the first, not the only one;
a `table-min-rows` shape (a Considered-options-style table naming at least N alternatives) is a
plausible second, not designed here.

### 4. `urzua doctor` verifies template ↔ config agreement

A new `doctor` check: for each configured record type with `required_sections`, does its template
(if one exists) contain a heading for every declared section name? Drift becomes a reported defect —
"template for `adr` is missing a `## Consequences` heading declared as required in config" — rather
than an assumption nothing verifies. This is the chosen answer to §2's "how do template and config
stay in agreement" question: verified, not generated, not merely hoped for.

### 5. Resolves RFC-13 via a decisionless-status exemption

RFC-13 ("a vacated identifier is a record, not a gap") named the problem without a mechanism: a
tombstone record for a vacated/withdrawn number never made a real decision, and requiring a
Y-statement or populated required sections from it forces it to invent content that lies to satisfy
the checker. Propose a configured set of statuses (e.g. `Withdrawn`) exempt from `required_sections`
and shape checks entirely — a record in a decisionless status is examined by nothing this RFC adds.

## Open questions

- **Resolved by ADR-33, after this RFC was drafted**: blockquote and bold-list are deprecated
  (not removed) in favor of `yaml-frontmatter`, on the strength of RFC-5's claim-graph nesting
  requirement — a flat, one-line-per-field shape has no path to representing that structure. Parsing
  support for the deprecated shapes stays (evaluation-before-adoption still works unmodified); a
  non-blocking `header.deprecated-shape` check is the mechanism. A general `migrate header-shape`
  command was considered and deliberately *not* committed to — no adopter is under any actual
  pressure to migrate anything while parsing support stays permanent, so building one now would be
  speculative capability with no evidenced demand yet.
- **This repo's own corpus migrates to `yaml-frontmatter`** via a one-time, unshipped conversion
  pass (ADR-33) — no longer a separate open question, folded into the same decision.
- What other content shapes are worth building beyond `y-statement` (a table-row-minimum check was
  named as a plausible second) — no real case has demanded one yet, so none are designed here.
- Should `doctor`'s new template/config check also catch the separately-found `header_shape`-vs-
  template-existence bug (`urzua new` prefers an existing template over the configured shape
  unconditionally), or is that a distinct, smaller fix outside this RFC's scope? Leaning toward
  fixing it as its own small change, tracked separately, not folded into this RFC's implementation.

## Non-goals

- Whole-document YAML storage (considered in the motivating discussion, rejected as the wrong fix
  for prose-section checkability specifically — RFC-16/ADR-17's header-only YAML frontmatter is
  unaffected by and orthogonal to this proposal).
- A generalized template-generation system (config as the only source, template auto-derived) —
  rejected in §2 for losing or duplicating hand-curated prompt text.
- Migrating any existing record's header shape or section content. This RFC adds a new checkable
  dimension; it does not rewrite anything that exists today.

## References

- RFC-10 — the closed-header model this proposal extends one level down to sections.
- RFC-13 — the vacated-identifier problem this RFC's decisionless-status exemption resolves.
- RFC-16/ADR-17 — the header-shape work this proposal is orthogonal to, not a revision of.
- SPEC-2 — "required sections per profile," named and unbuilt until this RFC.
- ADR-27 — `urzua new`'s template-vs-synthesis logic, and the header_shape/template-priority bug
  found while designing this RFC (§ Open questions).
