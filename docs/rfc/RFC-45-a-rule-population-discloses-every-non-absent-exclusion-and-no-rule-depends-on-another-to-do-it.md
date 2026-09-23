---
Stable-Id: 01M36B85BTBW86WVKZRCEY1JXM
Status: Accepted
Date: 2026-09-23
Author: beauwilliams
---
# 45 — a rule population discloses every non-absent exclusion, and no rule depends on another to do it

## Summary

Two related gaps in `Population`'s disclosure, found investigating one question: three review rounds
flagging the same "declared, not voted" narrowing as a false-positive regression turned out to be a
symptom of a broader principle this codebase has never stated or enforced — **no rule may depend on
another rule being enabled to cover a hole in its own disclosure.** `Population` today discloses three
numbers (`eligible`, `examined`, `out_of_scope`); everything else — a target's type never declaring a
field (`ADR-60`'s gate), and a header that flat-out didn't parse (`Outcome::Unreadable`) — collapses
into one undisclosed remainder, on the unstated assumption that some *other*, independently opt-in
rule will surface it. `ADR-53` guarantees no such pairing. This RFC makes both kinds of exclusion
disclosed and self-sufficient per rule, closing a reproduced, live instance of `ADR-55`'s core defect
class (`BUG-125`) along the way.

## Motivation

**Part 1 — the declared-not-voted ambiguity (originally filed).** `pointer_target_status`,
`narrative_field_stale`, and `claim_status_agreement` each read a target record's field through
`declared_cross_record_value`, which returns `Outcome::Absent` for two structurally different reasons:
the target wrote nothing, or the target's *type* never declared the field at all (`ADR-60`'s gate).
Collapsed into one bucket, a reviewer (or a human) sees a rule's population narrow and cannot tell
policy from regression without reading an ADR.

**Part 2 — a live, reproduced defect, not a hypothetical (`BUG-125`).** Investigating whether folding
`Absent`/`Unreadable` together was itself a problem surfaced a real one: `field.quality`'s own doc
comment justifies not disclosing `Unreadable` separately because *"`header.required-fields` reports
the parse error itself."* That assumes `header.required-fields` is enabled. It doesn't have to be —
`ADR-53` makes every rule independently opt-in, and nothing enforces the pairing. Reproduced on `main`:
a config enabling only `field.quality`, checked against a record with a completely unparseable
header, reports `"status": "ok"`, zero findings — `ADR-55`'s exact defect class, live, through an
entirely ordinary config.

**The shared principle, stated for the first time:** a rule's `Population` must be correct and
self-sufficient under *any* subset of `ALL_RULES` a repository enables, because `ADR-53` guarantees no
particular subset. Any rule whose honesty depends on a sibling rule's config is an undeclared,
unenforced coupling between two things the architecture says are independent. This generalizes past
`field.quality` — `field.pending`, `field.untrimmed-value`, `header.field-case-mismatch`, and any
other rule reading `record.header.get(...)` on a declared slot likely shares the same shape, not yet
individually verified.

## Proposal

**Part 2 fits `Population`'s existing model cleanly; Part 1 does not, discovered mid-implementation,
and was designed twice before landing.**

In `field.quality`-shaped rules, `Outcome::Unreadable` is the top-level verdict `census`/
`census_records` receives for a candidate — it slots directly into `Population`. But in
`pointer_target_status`/`narrative_field_stale`/`claim_status_agreement`, the candidate `census`
tracks is the *source* record's own declared slot, which is genuinely `Examined` the moment the
source's own field is read. Whether the *target* a reference resolves to can be judged is a nested,
secondary question underneath that candidate. Threading a new `Outcome` variant into `Population` for
this would mean inventing a population-inside-a-population concept nothing else in this codebase's 30
rules has, for three rules — the speculative-structure trap `AGENTS.md` already warns against. A first
revision proposed bolting a `Finding` onto each of the three rules' own private `continue` instead —
rejected on review as the same anti-pattern one level down: three rules independently checking the
same cross-record concern is exactly the "every caller invents its own check" shape
`header.field-case-mismatch` (`RFC-40`/`ADR-58`) already exists to prevent for a declared-field case
mismatch.

**Final proposal, Part 1: one new, dedicated rule — `relation.target-status-undeclared`.** For every
resolved pointer/narrative reference in the corpus, does the target's type declare `Status` at all
(`ADR-60`'s gate)? One rule, one `Population`, one place this is diagnosed — the consuming rules
(`pointer_target_status`, `narrative_field_stale`) are unchanged, keeping their original silent
`continue` on an undeclared target, because whether that gap is worth surfacing is no longer their
job. A future rule reading any other cross-record field inherits this coverage for free, the same way
no new field-reading rule needs its own case-mismatch check today. `claim_status_agreement`'s
claim-sourced references are a distinct data shape (external files, not corpus-declared slots) and are
explicitly out of this rule's scope — see Non-goals.

**Part 2, unchanged from the original proposal:** `Unreadable` becomes its own disclosed `Population`
count, computed structurally by `census`/`census_records`, closing `BUG-125` for every rule whose
`Outcome` body can return `Unreadable`, in one change. `Absent` stays the implicit remainder
(`eligible - examined - out_of_scope - unreadable`) — the one state that is self-evidently benign on
its own terms. `Population`'s constructor (`Population::detailed`) grows from four positional `usize`
arguments to five (`eligible`, `examined`, `out_of_scope`, `unreadable`); every existing call site is
touched, verified by the compiler (a missing argument is a build failure), not assumed complete.

## Open questions

- **`claim_status_agreement`'s own undeclared-target gap stays open.** The new rule covers
  pointer/narrative references only; a claim file claiming to close a record whose type never declares
  `Status` still silently skips, same as before this RFC. Not evidenced as a live problem yet — filed
  as its own follow-up if it is.
- **Does every rule that can produce `Unreadable` need auditing individually** to confirm it no longer
  relies on a sibling rule, once the count is disclosed, or is disclosure alone (an adopter can now
  *see* the number and act on it) sufficient without an inline behavior change per rule? Leaning
  toward disclosure being the fix itself.

## Non-goals

- Does not touch the population-construction-time exclusion (`declared_slots`/`field_slots`), already
  correctly disclosed via `eligible`'s own definition — unchanged by this RFC.
- Does not mandate that any two rules be enabled together — the fix is disclosure, not a new
  dependency; `ADR-53`'s "declared, not voted" stays exactly as strong, applied to `Population`'s own
  shape now instead of undermined by it.
- Does not audit every one of the 30 rules individually for other undisclosed cross-rule assumptions
  beyond `BUG-125`'s own instance — further instances are their own bugs if found.
- Does not extend `relation.target-status-undeclared` to `claim_status_agreement`'s claim-sourced
  references, or to any relation role beyond `Status` (`Embodiment`, `Supersession`) — no evidenced
  case for either yet (`AGENTS.md`'s "don't build speculative capability").
- Does not add a new `Outcome` variant or touch `Population`'s shape for Part 1 at all — resolved to a
  new rule instead, after two narrower designs were considered and rejected.

## References

- `ADR-60` — the cross-record declaration gate Part 1's ambiguity lives inside.
- `ADR-53` — "declared, not voted"; both parts of this RFC are that principle applied to
  `Population`'s own disclosure rather than assumed to hold by convention.
- `ADR-55` — the defect class `BUG-125` is a live instance of.
- `BUG-125` — the reproduced, live defect this RFC's Part 2 closes.
- `RFC-40`/`ADR-58` — the `header.field-case-mismatch` precedent: one dedicated rule, not every
  caller inventing its own check — the shape Part 1's final design follows exactly.
- `AGENTS.md`'s "A narrower rule population is usually policy, not a regression" section — the prose
  stopgap this RFC's implementation supersedes and removes.
- `MILE-106` — built a narrower gate (`config.scope-matches-nothing`, checking only `out_of_scope`)
  that does not catch `BUG-125`'s shape; this RFC's disclosure is what a future, stronger version of
  that gate would need to act on.
- `MILE-80` — the separately-tracked "richer severity taxonomy" question this RFC deliberately
  doesn't reopen; `relation.target-status-undeclared` uses `Warning`, an existing tier.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed as `Draft`. **Why:** three consecutive review rounds flagged the same "declared, not voted" exclusion as a false-positive regression; the user pushed back that a prose fix (`AGENTS.md`) is weaker than a structural one. | **substantive** |
> | 2026-09-23 | Broadened significantly after the user asked whether silently folding `Absent`/`Unreadable` was itself a design problem. Investigating surfaced `BUG-125`, a live, reproduced instance of `ADR-55`'s defect class, and the user stated the general principle directly: no rule may depend on another rule being enabled to cover its own disclosure gap. Retitled and rescoped from a narrow three-rule fix to `Population`'s general disclosure discipline. | **substantive** |
> | 2026-09-23 | Accepted via `ADR-63`: `Population` gains an `unreadable` count (Part 2), fixed once in shared `census`/`census_records` machinery. Part 1 initially decided as a new `Outcome::Undeclared` variant. | **substantive** |
> | 2026-09-23 | Part 1 redesigned twice more, same day, before implementation landed. First: found mid-build that `Outcome::Undeclared` can't actually reach `Population` for the three affected rules (the ambiguity is nested one level below the census candidate) — reverted, replaced with a `Finding` bolted onto each of the three rules' own `continue`. Second, on review: that repeats "every caller invents its own check" one level down from what `header.field-case-mismatch` already prevents for case mismatches — replaced with one dedicated rule, `relation.target-status-undeclared`, covering all three consuming rules' shared concern in one place. `Outcome::Undeclared` removed entirely; unused once the new rule made it unnecessary. | **substantive** |
