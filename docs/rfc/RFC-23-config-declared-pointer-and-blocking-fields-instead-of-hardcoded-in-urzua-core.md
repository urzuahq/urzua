---
Stable-Id: 01M21FK8QA0D6QJQWHKBND90WB
Status: Accepted
Date: 2026-09-08
Author: beauwilliams
Implements: BUG-8
---
# 23 — Config-declared pointer and blocking fields, instead of hardcoded in `urzua-core`

## Summary

`pointer_resolution`, `blocked_on_stale`, and `header.pointer-field-clean` each hardcode a fixed
list of field names (`Implements`/`Derives-from`/`Parent`/`Blocked-on`) directly in Rust source,
rather than reading them from `.urzua/config.toml` the way `header_shape`/`known_fields`/`spec` are
already declared per type. Propose making the field lists themselves configurable, closing the real
gap `BUG-8` names.

## Motivation

Found live, discussing whether an adopting org could add their own relationship vocabulary (e.g. a
field named `Feeds-into`, or `Realizes`, or `DependsOn`) the way they can already add their own
record types via config with zero `urzua-core` changes. They can't, today: `pointer_resolution`
scans a fixed array; a field outside it is invisible to resolution checking, dangling-reference
detection, and `urzua graph`, no matter what `known_fields` declares. This directly contradicts
`BUG-8`'s finding that the README's own pitch overstates this as already working.

The gap is sharper than "one list should be config": there are genuinely **two different kinds** of
pointer field already implicit in the code, never named as such:

1. **Plain resolving pointers** (`Implements`, `Derives-from`, `Parent` today) -- exist, resolve to
   a real record, nothing else checked.
2. **Staleness-aware, prose-tolerant pointers** (`Blocked-on` today) -- resolve *if* a reference is
   present, tolerate free text alongside or instead of one, and additionally check the target's
   `Status` for terminal-ness.

Making only the *list* configurable without naming this distinction would let an org declare a
field as "pointer-like" without saying which behavior it should get -- `header.pointer-field-clean`
would then have no way to know whether to enforce cleanliness (kind 1) or tolerate prose (kind 2).

## Proposal

Two new per-type config concepts, declared the same way `known_fields` already is -- no global
list; each type declares its own, independently:

```toml
[record_types.rfc]
# ...existing fields...
pointer_fields = ["Implements", "Derives-from", "Amends"]
narrative_fields = ["Blocked-on"]
```

- **`pointer_fields`**: resolved by `pointer_resolution` (existence-checked, `urzua graph` edges
  labeled by field name), and enforced clean by `header.pointer-field-clean` -- any comma-separated
  entry that isn't exactly a reference token is a finding, no exceptions.
- **`narrative_fields`**: resolved by `pointer_resolution` too, but *not* subject to the
  clean-field check, and separately checked by `blocked_on_stale` for target terminal-status.
  `extract_references`'s existing lenient, leading-token extraction is exactly right for this kind
  and stays unchanged. Named for the mechanism (tolerates narrative text alongside an optional
  embedded reference), not today's one use case -- `blocking_fields` was considered and rejected as
  a name, since it would misdescribe a future narrative-tolerant field that isn't about blocking at
  all (e.g. a hypothetical `Motivated-by`).

**No default, no backward-compatibility fallback.** A type that declares neither list has zero
fields of either kind checked -- `examined` stays `0` for it, the same "declared, not inferred"
shape `header_layout`/`known_fields` already use when undeclared. Deliberately not treating today's
hardcoded list as an implicit default: this tool has no real external adopters yet to preserve
behavior for, and carrying forward a compatibility shim nobody needs would just be unaudited cruft
from day one. Every one of this repo's own six types will need an explicit declaration once this
ships -- a real, visible migration, not a hidden default silently doing the same job.

**`urzua graph` gains a `kind` field on every edge**, distinguishing which list produced it:

```json
{"from": "MILE-6", "relation": "Implements", "to": "RFC-18", "kind": "pointer", "dangling": false}
```

## Open questions

- **README correction scope** -- once this lands, is the `Feeds-into` example in the lineage section
  finally true as written, or does it still need adjusting for some other reason found along the way?
- **This RFC's own header is a live instance of the gap it names.** `Implements: BUG-8` above
  resolves correctly (`pointer_resolution` doesn't care that `rfc`'s `known_fields` never declared
  it), but `header.field-set-consistency` separately flags it as undeclared -- deliberately left
  firing rather than quietly silenced by adding `Implements` to `rfc`'s `known_fields`, since
  whether `rfc` *should* declare it is exactly the kind of small, real decision this RFC's own
  proposal would make legible instead of ad hoc. Left open, not decided here.
- **Are there other backward-compatibility shims already in `urzua-core`** that exist only for
  imagined future adopters rather than any real one? Raised live alongside this RFC's own
  no-defaults decision; scoped as a separate audit, not answered here -- the audit landed as BUG-9
  (legacy pre-`ADR-36` filename support), filed in the same PR as this RFC.

## Amendment (2026-09-09): four gaps CodeRabbit found in this proposal, resolved

Review on the PR that filed this RFC found four real, unanswered questions in the design above.
Resolved:

1. **A type can silently omit either list, or both.** `pointer_fields`/`narrative_fields` as plain
   `Option<Vec<String>>` config keys means a type that forgets to declare one of them (or both) is
   indistinguishable, at the config level, from a type that deliberately declares zero fields of
   that kind -- all three read as "examined stays 0" for whatever's missing. That's a real
   regression from this RFC's own "declared, not silently inferred" bar: undeclared should fail
   *visibly*, not just skip quietly. **Resolution:** a new check, `config.pointer-declaration-missing`
   (Error), fires when a configured type does not have **both** keys present in
   `.urzua/config.toml` -- declaring only `pointer_fields` and omitting `narrative_fields` fires,
   the same as omitting both. An explicit empty array (`pointer_fields = []`) is a real declaration
   and does not fire on its own; only an absent key does, on either side.

2. **Relationship to `known_fields` was undefined.** A field named in `pointer_fields`/
   `narrative_fields` but not also in that type's `required_fields`/`known_fields` would resolve
   correctly (`pointer_resolution` doesn't consult `known_fields`) while `header.field-set-
   consistency` simultaneously reports it as an undeclared field -- two config lists disagreeing
   about whether the same field is legitimate. Considered making membership in `pointer_fields`/
   `narrative_fields` automatically satisfy `known_fields` (no duplicate bookkeeping) and rejected
   it: `known_fields` should stay the single, explicit source of truth for "which fields this type's
   header may carry," full stop -- not something a field can back into by being named as a pointer
   elsewhere. **Resolution:** a new check, `config.pointer-field-not-known` (Error), fires when an
   entry in `pointer_fields`/`narrative_fields` isn't also present in that type's `required_fields`
   or `known_fields`. Declaring a pointer field costs two config lines, not one -- the redundancy is
   the point, not an oversight.
3. **Overlap between the two lists was undefined.** A field named in both `pointer_fields` and
   `narrative_fields` for the same type would be ambiguous about which validation behavior (clean-
   only vs. prose-tolerant) applies, and could double-emit `urzua graph` edges. **Resolution:** a
   new check, `config.pointer-narrative-overlap` (Error), fires when the same field name appears in
   both lists for one type. Mutually exclusive, no precedence rule needed.
4. **Whether `narrative_fields` are inherently staleness-checked was unstated.** Re-reading ADR-44's
   own Context section, the category was already defined as "staleness-aware, prose-tolerant
   pointers" -- staleness-checking is what distinguishes a narrative field from a plain pointer
   field, not a `Blocked-on`-specific side effect layered on afterward. This RFC's own Proposal
   section already said as much ("separately checked by `blocked_on_stale` for target
   terminal-status," stated for the category, not just `Blocked-on`); its later `Motivated-by`
   example argued against naming the category `blocking_fields`, never against staleness-checking
   itself -- a `Motivated-by` pointing at a since-`Rejected` record is exactly as worth flagging as a
   `Blocked-on` pointing at an already-`Fixed` bug. **Resolution:** no mechanism change and no new
   design decision here -- this entry exists only to state explicitly, in one place, what the
   Proposal section had already implied but never said in so many words.

`Motivated-by` itself remains unbuilt and undecided -- named here only as the example (from this
RFC's own original Proposal section, not this amendment) that surfaced question 4, scoped out to
RFC-24 per this project's "add a field once a pattern recurs" discipline.

## Non-goals

- **Does not implement a third pointer-field kind** beyond the two identified -- if a real case
  needs something else later, that's a separate RFC, per this project's own "add a field once a
  pattern recurs" discipline.
- **Does not retroactively migrate this repo's own config** -- covered by whatever milestone
  implements this RFC, not decided here.
- **Does not perform the broader backward-compatibility-shim audit** named above -- separate work.

## References

- BUG-8 -- the concrete defect this RFC's `Implements` pointer closes: the README's overstated
  claim, and the underlying hardcoding it was wrong about.
- ADR-38/39/43 -- the "declared, not voted or inferred" precedent this proposal extends to pointer
  fields.
- RFC-20 -- the CLI/command taxonomy RFC, whose own table already asserts pointer fields are
  "plain field names an org declares" -- this RFC is what would make that literally true.
- RFC-22 -- a related, separately-filed idea (a `blocker` record type) that would sit on top of
  whichever design this RFC lands on, not a substitute for it.
- MILE-90 -- the milestone that builds this RFC's design, including the three validation checks
  this amendment adds.
- RFC-24 -- scopes out the `Motivated-by` field this amendment's naming argument used as its
  example, without deciding whether to build it.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial RFC. **Why:** found live discussing whether an adopting org could add its own relationship vocabulary the way it can already add record types -- it can't, today, closing exactly the gap BUG-8 names in the README's own claim. | **structural** |
> | 2026-09-08 | Design settled through discussion: per-type only, no global list; renamed `blocking_fields` to `narrative_fields` (names the mechanism, not today's one use case); no backward-compatibility default -- an undeclared type gets zero fields of either kind checked, matching `header_layout`/`known_fields`'s own undeclared-means-skip precedent; `urzua graph` gains a `kind` field per edge. Config key names, per-type-vs-global, and backward compatibility were all previously open questions -- now decided; README correction scope and this RFC's own undeclared `Implements` field remain open. | **substantive** |
> | 2026-09-09 | Amendment: resolved four gaps found on review -- added three new validation checks (`config.pointer-declaration-missing`, `config.pointer-field-not-known`, `config.pointer-narrative-overlap`) and clarified that `narrative_fields` are inherently staleness-checked, no mechanism change needed. | **substantive** |
