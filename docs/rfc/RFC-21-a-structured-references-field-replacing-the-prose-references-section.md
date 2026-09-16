---
Stable-Id: 01M215TJWNXAJRZ30G6Z7TQBPF
Status: Rejected
Date: 2026-09-08
Author: beauwilliams
---
# 21 — A structured References field, replacing the prose References section

## Summary

Add a generic, optional, comma-separated `References` header field to every record type, resolved
by `pointer.resolution` the same way `Implements`/`Derives-from`/`Parent` already are. Retire the
`## References` prose section from every template — once the structured field exists, the section
only repeats the same IDs with a one-line annotation, and a real audit shows citations were never
actually confined to it anyway.

## Motivation

Found live, twice: `SPEC-16`/`SPEC-17` cited RFC-1 in `## References` prose for a while before
gaining a real `Derives-from`/`Implements` header pointer; then `RFC-18`'s `## References` said
"MILE-6 — ... this proposal folds in" with no reciprocal structured pointer on either record until
asked about directly. Neither was caught by anything, because `## References` is body prose, never
parsed as data.

An audit across all 166 `adr`/`rfc`/`spec`/`bug` records, counting record-ID citations by section
(excluding `## References` itself), found citations are not actually confined to a References-style
section at all — they're everywhere: `Context` (37 files), `Decision` (28), `Consequences` (27),
`Open questions` (19), `Proposal` (17), `Motivation`/`Non-goals` (14 each), `Summary`/`Purpose` (13
each), and ~40 more headings with 1-5 hits each. `## References` was never the actual home of most
citations in practice — it was one convention among many places a record happens to mention another
one, which makes it a weak place to anchor any mechanical check.

## Why rejected

Both of this RFC's own motivating cases turned out not to need a new field at all, once traced
through carefully rather than assumed:

- **`SPEC-16`/`SPEC-17` citing `RFC-1`** in prose was fixed by adding `Implements: ADR-10` to their
  own headers — an *existing* field, pointing at `ADR-10` (the decision record for `RFC-1`), not at
  `RFC-1` itself; `pointer.resolution`/`urzua graph` produce an `ADR-10` edge, correctly.
- **`RFC-18` citing `MILE-6`** was fixed by adding `RFC-18` to `MILE-6`'s *existing* `Implements`
  field. Checked live in `urzua graph` after: `MILE-6 --Implements--> RFC-17` and
  `MILE-6 --Implements--> RFC-18` both appear, `dangling: false` — the relationship is fully
  represented, from the milestone's own side, with zero schema change.

Neither case was ever "missing vocabulary." Both were "nobody had gone looking for which existing
typed field already fit, and in which direction." That's a *detection* problem — recognizing a
citation is load-bearing and pointing whoever's writing it at the right existing field — not a
schema problem. `MILE-86` (this RFC's own stated origin) already named that framing before this RFC
drifted toward inventing a new field instead; rolled back to it.

What's left over once both real cases are accounted for by existing fields is genuinely thin: only
citations with no nameable relationship at all (e.g. "the rule whose live finding this spec
closes"). Those were already agreed, earlier in this same discussion, to be legitimately loose —
not worth a mechanical check, let alone new schema and a template rewrite across all six types just
to make that narrow residual case checkable.

**Superseded by**: `MILE-86`, restored to its original "detect the gap, prompt for an existing
field" scope rather than this RFC's "build a new field" one.

## Proposal

- **A new field, `References`**, comma-separated, added to `known_fields` for every configured
  type (`adr`, `rfc`, `spec`, `milestone`, `bug`, `waiver`) — resolved by `pointer_resolution`
  exactly like `Implements`/`Derives-from`/`Parent` today (reusing the same generic, field-name-
  agnostic scan `SPEC-13` already describes as shared infrastructure).
- **Optional, not required.** Forcing a value into a field with nothing legitimate to reference
  would manufacture exactly the failure mode `field_state::classify` exists to catch elsewhere
  (`Placeholder` vs. real content) — the same reason `Implements` stays optional on `milestone`
  today ("a milestone can exist before a decision does," SPEC-6). Declared, not voted or forced,
  same principle as `header_layout`/`known_fields`/`spec` (ADR-38/39/43).
- **`## References` removed from every `.urzua/templates/*.md`.** The prose section's actual job —
  telling a reader what this record relates to — is fully covered by the structured field once one
  exists; a one-line "why" per citation duplicates reasoning that belongs in whichever real section
  (`Context`, `Decision`, `Consequences`) already discusses that relationship, not a separate list
  bolted on at the end.
- **Existing records are not retroactively migrated by this RFC.** Deciding this shape is separate
  from the (larger, `docs`-corpus-wide) work of converting every existing `## References` section
  into the new field — named here as necessary follow-up, not committed to in this proposal.

## Open questions

- **Does this field need its own name, or should it reuse `Derives-from`'s existing "comma-
  separated, optional" shape more loosely?** A new field keeps `Implements`/`Derives-from`/`Parent`'s
  precise, specific relationship-type meaning intact (each labels a `urzua graph` edge with what
  kind of relationship it is); reusing an existing field for looser citations would blur that
  signal. Leaning toward a new field, not fully closed.
- **What happens to the "why" a citation mattered**, currently carried by `## References`'s one-line
  annotations, once the section is gone? The proposal assumes that reasoning already belongs (or
  should be moved) into whichever real section discusses the relationship — untested against the
  full corpus's actual content, which may reveal citations whose "why" doesn't fit naturally
  anywhere else.
- **Should a finding ever nudge an author toward filling this field in**, given it's optional and
  intentionally not derived from prose? MILE-44 ("suggested action on every finding") is the natural
  home for that idea — e.g., a finding on a *different*, already-blank required field could suggest
  "review this record's own reasoning sections for what else it should cite" — but this RFC doesn't
  propose *when* or *how* that fires, only that the underlying field this would point at exists.
- **Migration scope and sequencing** for retrofitting the existing corpus's `## References` content
  into the new field — a real, likely large unit of work (166 files), not scoped here.

## Non-goals

- **Does not attempt to mechanically derive `References`'s value from prose.** The field stays
  author-declared, the same "declared, not inferred" principle used throughout this corpus's schema
  decisions — a keyword scan for citation-shaped language was already shown (via the audit above) to
  be too noisy to use as a source of truth.
- **Does not migrate the existing corpus.** Scoped as explicit follow-up.
- **Does not decide MILE-44's finding-message design.** Named as a natural connection point, not
  specified here.

## References

- ADR-38/39/43 — the "declared, not voted or inferred" precedent this field's optionality follows.
- SPEC-6 — `Implements` staying optional on `milestone`, the direct precedent for this field's own
  optionality.
- SPEC-13 — `record_id`/`extract_references`, the shared machinery this field's resolution reuses.
- MILE-6/RFC-18 — the concrete case (a load-bearing citation in prose, no reciprocal pointer) this
  RFC is a response to.
- MILE-44 — configurable suggested actions on findings, the natural home for nudging an author
  toward this field, not decided here.
- MILE-86 — names the same underlying gap; this RFC's structural approach is rejected in favor of
  MILE-86's original detection-based one (see "Why rejected" above).
