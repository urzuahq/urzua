# 0010 — The record header is a closed structure, validated as a whole

> Status: Accepted
> Date: 2026-08-11
> Author: (session author)
> Supersedes / Superseded-by: —

## Summary

A record-linter that validates a header **field by field** — is `Status` present, is `Date`
well-formed, is `Deciders` non-blank — never validates the header **as a structure**: a bounded
region containing a known set of keys, each appearing exactly once, and nothing else. Propose that
Urzua treat the header as a closed structure with a single shared parser: the region is located by
anchoring to the metadata shape rather than by a positional slice, unknown keys are reported rather
than ignored, a repeated key is an error rather than first-wins, and a field the parser could not
find is always an emitted finding rather than silence. The negative space — *"and nothing else"* —
is not expressible as a per-field check, which is why independently built linters, on unrelated
corpora, each miss it in a different way.

## Motivation

Independently built linters tend to hit this gap in different forms. None find it by
design review; each finds it by being bitten.

**The full arc, already run.** One linter searched the whole document for a `Status:` label. A
status-tracking bullet list in body prose matched instead of the real header field, on the wrong
side of a search-order coincidence. The fix was to narrow the search to everything before the first
`##` heading. **That fix introduced a worse bug:** a corpus layout the org's own agent instructions
explicitly blessed — a dated supplement placed above the metadata — put a `##` on line 3, reducing
the header slice to 58 characters. Both "blocking" checks then did nothing at all, silently. The
same regex also never recognized the table-shaped header used elsewhere in the corpus. Net: **6 of
28 specs, including two security contracts, passed with zero validation and no signal that anything
had been skipped** — a spec could carry `Status: Frobnicated` and `Date: last Tuesday` and pass
clean. The eventual fix abandoned positional narrowing entirely in favour of anchoring directly to
the two metadata shapes the corpus actually uses, extracted into a shared module used identically by
every linter that needed it.

**An unknown key silently tolerated.** In a second corpus, an agent invented a header field
mid-edit. The parser read only the keys it knew and ignored the rest, so there was no lint error, no
malformed index entry, and no signal of any kind. A record could carry an invented field that no
tool reads and every reader trusts.

**The body satisfies the header, and duplicates resolve silently.** In a third corpus, a field regex
anchored to the list-item shape was still satisfied by a list item in the *body*, because the body
is made of list items too — the corpus contained body bullets in exactly the header's
`- **Key:** value` shape (`Precedent:`, `Location:`, `Cost:`, `Migration:`). A second occurrence of a
key resolved to the first, so a record could state two different acceptance dates and the linter
would report on only one of them.

**The unifying claim:** every one of these is a failure to answer *"is this the header, all of it,
and only it?"* — a question about the region and its closure. Per-field presence checks cannot ask
it, no matter how many fields are checked or how carefully each regex is written.

**A demonstration of why this needs an RFC rather than a fix.** The third corpus's most recent
change, at the time it was reviewed, had adopted the slice-at-first-`##` approach for one new
field — the exact intermediate step the first corpus had already proved dangerous. That corpus was
safe only by accident: an early `##` collapses its header slice to 18 characters, but the check
fails *loud* rather than silent, because `Status` is still located by a whole-document regex while
the new field uses the slice. **Two inconsistent scoping strategies disagreeing is what produces the
error.** Make them consistent — the obvious cleanup — and the failure becomes the silent one from
the first corpus.

## Proposal

### 1. One header parser, shared by every consumer

A near-identical bug already establishes the invariant: a governance linter should have exactly
**one** parser for a given construct, and every check routes through it. The first corpus's eventual
fix converged on the same shape independently — one shared module, used identically by every linter
that needed it. Header parsing is the same case: one function returns the parsed header, and no
check may run its own regex over raw document text.

The parser's output is a structure, not a string — an ordered list of `(key, value, line)` plus the
region's own boundaries — so that closure questions have something to be asked about.

### 2. The region is found by anchoring to the metadata shape, never by a positional slice

Both positional heuristics are documented failures: whole-document search matches body prose (the
first corpus's original bug, and the third corpus's body bullets), and slice-at-first-heading
silently truncates (the first corpus's follow-up bug). Neither is a tuning problem; both are the
wrong kind of rule.

Instead: the header is the **maximal contiguous run of metadata-shaped lines** anchored at the first
such line, where "metadata-shaped" is declared per profile (RFC-0001 §1a) rather than hardcoded — one
corpus alone uses a blockquote form and a table form together, so a single shape is already known to
be insufficient. A document with no such run has **no header**, which is a reportable finding, not
an empty result.

### 3. Closure: exactly the known keys, exactly once each, nothing else

Three rules the current per-field model cannot express:

- **Unknown key → finding.** Warn-level by default with an explicit, config-declared extension
  namespace as the auditable opt-out — never blanket tolerance. The opt-out shape should be default
  strict, declared rather than inferred.
- **Repeated key → error.** Never first-wins. Two values means the record states two facts and no
  reader can tell which is operative.
- **Key outside the header region → finding, not satisfaction.** A `- **Status:** ...` line in the
  body is a documentation example or a decoy; it is never the field.

### 4. "Field not found" is always emitted, never silence

The sharpest finding from the arc above is not that its checks were wrong — it is that **6 of 28
specs passed with zero validation and nothing said so.** Any header field the parser could not locate produces a
finding, including when the reason is that the header itself was not found. This is the same
non-negotiable already written into SPEC-0001 (`check` must distinguish "found nothing" from "ran
on nothing"), applied one level down: a *field* lookup that silently returns nothing is the same
defect as a *check* that silently examines nothing.

### 5. Consequence for RFC-0001

The core schema becomes genuinely closed rather than conventionally closed: `type` declares a
profile, and the profile declares the complete legal key set for that record's header. RFC-0001 §1a
already made profiles declared rather than enumerated; this makes their key sets *binding* rather
than documentary.

### The recogniser must be more permissive than the validator

A closed structure needs two separate things: something that *finds* a field, and
something that *judges* whether the field is legal. Where one expression does
both, illegal input becomes unreportable by construction — the matcher fails to
match, and the checker reports nothing.

An implementation of exactly this proposal got it wrong twice. Its row matcher
accepted field names of letters, spaces and slashes, so `Related issue(s)` — one
of the invented synonyms closure exists to prevent — was silent rather than
rejected. Widening the character class moved the boundary without removing it;
ten realistic names were still invisible afterwards. The fix is to capture
anything that is not a structural delimiter and let the declared set decide.

The same principle applies to locating the header at all. Stripping fenced
blocks before parsing was correct in intent and wrong in four ways at once
(`~~~` fences, indented fences, longer fences wrapping shorter, unterminated
fences), which made *correct* records fail. So closure needs both properties
tested, and they pull in opposite directions.

**The single property worth asserting is neither "the right things are accepted"
nor "the wrong things are rejected". It is that no input is ever silently
ignored:** for any generated field name, the outcome is acceptance under a
declared name or a report — never nothing. That one property catches both
directions, and it is cheap to fuzz.

## Open questions

- **How is the extension namespace declared** — a reserved prefix (`x-`), an explicit allowlist in
  config, or per-profile schema extension? A prefix is cheapest and self-documenting in the record
  itself; an allowlist is more auditable. Undecided.
- **Does the maximal-contiguous-run rule survive a blank line inside a header?** Real corpora can
  contain wrapped multi-line values, and at least one header shape observed elsewhere separates
  groups of fields. The rule needs a precise definition of "contiguous" tested against real records
  before it is trusted — the exact mistake the arc above made twice.
- **Is an unknown key warn or error by default?** Warn avoids a flood on adoption, but the failure it
  prevents — an invented field that looks authoritative — is precisely the kind that a warning
  stream buries. Possibly error-by-default with a documented migration path.
- **Where does the table-shaped header fit** — a second declared shape per profile, or a
  normalization step before parsing? At least one real corpus needs both a blockquote and a table
  form together, so this is not hypothetical.

## Non-goals

- **Validating header *values*** beyond shape and closure. Whether `Deciders` names a real person
  is a different check, already covered elsewhere.
- **Prescribing one header syntax across all corpora.** Different corpora use different shapes; the
  point is that each declares its shape, not that they converge on ours.
- **Body structure.** This RFC covers the header region only. Required-section checking is separate.
- **Retrofitting existing corpora.** Adoption sequencing is deployment work, not schema design.

## References

- RFC-0001 §1a — declared profiles, which this RFC makes binding.
- `docs/specs/0001-v0-cli.md` — the no-silent-no-op rule this RFC applies at field granularity.
