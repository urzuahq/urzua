---
Stable-Id: 01M33RT75Z3NJKPK7A3SHBF0FY
Status: Accepted
Date: 2026-09-22
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-40
---
# 58 — declared field lookups fail visibly not silently and are never fabricated

## Context

`ADR-57` decided a declared field *name* compares exactly. It never decided what a rule's *value read*
does when that exact-cased lookup misses, and two live rules answered the question in opposite,
undeclared ways:

`claim_status_agreement` (`BUG-100`) fabricated a sentinel on a miss and let it flow into a real
comparison: a `Status` field written `status:` made every claim citing that record a blocking `Error`,
with a message that reads as a status disagreement rather than a lookup miss.

The embodiment rule family (`BUG-101`) treated a miss as ordinary non-adoption and reported nothing:
a `Realized-by` field written `realized-by:` made a real `Embodiment` inconsistency unexaminable, with
`eligible > examined` the only trace — indistinguishable from a type that never adopted the field.

Both bugs are the same root cause reached in opposite directions, found in one `/code-review` pass
over everything landed for 0.4.0. `RFC-40` proposed a shared contract; this record decides it.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Fix each call site to `continue`/skip on a miss, independently | Smallest diff | The next rule reading a declared field guesses a third way; the actual gap (nothing says what a miss means) is untouched |
| A shared `read_declared` returning `Option<&str>` with a documented convention to always skip on `None` | One function, easy to write | A convention with no check behind it is what let `RFC-39`'s six comparison sites and this ADR's own two call sites diverge; `.unwrap_or(...)` is still reachable |
| A shared `read_declared` returning a 3-state `FieldRead` (`Present`/`Missing`/`Unreadable`), plus a dedicated rule that is the only place a case-mismatch is *diagnosed* | No sentinel to reach for; a miss is structurally distinct from a value; the near-miss diagnostic lives in one place instead of N | A fourth state (`MissingButCased`) was considered and rejected below |

A fourth option — folding a "written under a different case" state directly into `FieldRead` — was
considered and rejected: it would make every caller of `read_declared` decide whether to *also* report
the near-miss, reintroducing exactly the "guess per call site" problem this ADR exists to close. One
rule owns that diagnosis; every other rule just sees `Missing`.

## Decision

In the context of two rules answering "a declared field's exact-cased lookup missed" in opposite,
undeclared ways, facing a third rule that would inevitably guess a third way, **we decided that every
rule reading a declared field's value goes through `Header::read_declared`, which returns `Present`,
`Missing`, or `Unreadable` and never a sentinel a caller can silently propagate**, and that **a
differently-cased declared field is diagnosed by exactly one rule, `header.field-case-mismatch`, never
by the rule that happened to read it first**, to make a lookup miss and a value disagreement
structurally impossible to confuse, accepting that a small helper (`declared_value`) and a fourth rule
were added rather than patching two call sites directly.

**Decided:** `claim_status_agreement` and `pointer_target_status` (which shared `claim_status_agreement`'s
exact fail-open shape) skip on anything but `Present` — a miss is unjudged, not judged-and-wrong.

**Decided:** the embodiment rule family (`embodiment_locator_exists`, `embodiment_consistency`,
`embodiment_locator_promotion_candidate`) keeps reporting `Outcome::Absent` on a genuine miss, but now
distinguishes it from `Outcome::Unreadable` when the record's header didn't parse at all — a smaller
instance of the same conflation, visible once `read_declared` existed to separate the two.

**Decided:** `header.field-case-mismatch` is a new, declared, opt-in rule (`ADR-53`) over the same
`Field`-unit population as `field.untrimmed-value`, reporting any declared field whose exact-cased key
is absent but a case-insensitively matching key is present. This is the rule that closes `BUG-101`'s
silence and gives `BUG-100`'s fabricated sentinel something honest to report instead.

**Left open:** whether `read_declared` fully replaces `Header::get` at every remaining adopter-vocabulary
call site, or coexists with it at sites that already correctly treat a miss as legitimate absence with
no further diagnosis owed. `RFC-40`'s proposed lint (forbidding `.header.get(...).unwrap_or(...)`
outside `header.rs`) is not built in this pass; the two call sites this ADR fixes are migrated by hand,
and a planted-violation test covers `header.field-case-mismatch` itself but not a future misuse of `get`.

## Reversibility

Cheap to reverse: `read_declared` and `declared_value` are additive, `header.field-case-mismatch` is
opt-in like every rule (`ADR-53`), and removing it returns to the pre-`RFC-40` silence rather than to
a worse state. The two call-site fixes (skip-on-miss) are a narrower behavior change and would need
their own regression tests re-argued if reverted, since they close real, reproduced bugs.

## Consequences

A corpus with a mis-cased declared field now gets one honest diagnostic
(`header.field-case-mismatch`) instead of a fabricated blocking error in one rule and silence in
another. `claim_status_agreement` and `pointer_target_status` no longer flag a claim or a pointer
target as disagreeing with a status they never actually read. The lint from `RFC-40` remains future
work — a rule written tomorrow that reads a declared field via bare `Header::get` and invents its own
fallback is not yet mechanically caught, only caught by this ADR's own two fixes and this record's
existence as a search hit.

## References

- `RFC-40` — the proposal this ADR decides.
- `ADR-57` — the field-name exact-match decision this ADR fills a gap in.
- `BUG-100`, `BUG-101` — the two bugs this decision closes.
- `ADR-55` — the disclosure principle `header.field-case-mismatch` implements for this specific miss.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Decided and implemented in the same pass as `RFC-40`'s filing, given the small scope. **Why:** the fix was already correct and complete before the decision was written down; documenting after implementation risked the decision record becoming a description of what happened rather than a choice among real alternatives, so the rejected fourth option (a `MissingButCased` state) is recorded explicitly even though it was never built. | **substantive** |
