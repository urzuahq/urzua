---
Stable-Id: 01M6NE3QA7SY782EX298E5J9Q0
Status: Proposed
Date: 2026-11-05
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: —
Derives-from: —
---
# 86 — A waiver may be permanent when the repository declares which reasons allow it

## Context

`ADR-84` (Accepted) holds a record's `Status` to its declared evidence field: a `success` status whose evidence
computes to an `unevidenced_tiers` tier is a finding (`judge_delivery`, `rules/status.rs:182`, variant
`delivery-without-evidence`, `rules/status.rs:256`). It names a waiver (`ADR-11`) as the exemption route for a
done record that legitimately has no evidence. Enabling the rule on this repository's defect type (the build,
the open PR that builds it) shows the need: 24 `Fixed` records have no test to cite. the review of that PR classified
them: 2 have real tests that can be cited (`BUG-142`, `BUG-147`); 5 are refactors covered by existing suites
(105, 138, 140, 141, 144); 7 are docs, records or workflow only (10, 14, 15, 21, 26, 107, 189); 5 are
performance only (115, 116, 122, 134, 143); 2 had their test deleted by `RFC-35` (43, 54); 3 are genuinely
unevidenced (72, 90, 146). The last 3 are not candidates for any permanent exemption. The other 19 lack a test
for a reason that will not change.

**What the code does today, which corrects the framing that a waiver always needs an expiry.** `Expires` is
optional in the engine: `Suppression.expires` is an `Option` (`urzua-core/src/waiver.rs:55`) and `is_active`
treats `None` as active forever (`waiver.rs:71-73`). `SPEC-10` says an omitted `Expires` "asserts the exception
structural", and the waiver template says "omitted means permanent/structural" (`.urzua/templates/waiver.md`).
This repository's waiver type requires only `Rule`, `Scope` and `Reason` (`.urzua/config.yaml:342-344`), and
every one of the 9 waivers on file carries an `Expires` because its author chose to, not because anything
requires it. So the engine can already express a permanent waiver. What it cannot do is **govern** one:

1. **No vocabulary.** The only record of why an exception is permanent is free-text `Reason`. A permanent
   waiver for "performance-only" and one that is simply forgotten read the same, and a reviewer cannot ask
   `check` for the permanent ones by cause.
2. **No restriction.** A no-expiry waiver is allowed for any rule and for `Scope: *` (`waiver.rs:89`). A
   repository cannot say that a rule may be waived permanently only for stated causes, or that permanence is
   not available elsewhere. Without that, a permanent waiver is the cheapest way to silence any finding, which
   is why `ADR-11` pairs it with review.
3. **No count.** `waiver.matches-nothing` reports stale waivers (`rules/waiver.rs:28-134`) but nothing reports
   how many exceptions are permanent, or why.
4. **No citation.** A waiver names no record. `pointer_fields` is declared empty (`SPEC-10`), so "the test was
   deleted by `RFC-35`" is prose that no rule resolves or flags when `RFC-35` changes.

An expiry is the wrong tool for the 19: `WAIVER-0022` expires after a year "so the exception is revisited",
which is right for a temporary exception and false for a permanent fact. A bulk renewal each year is churn
that teaches reviewers to renew without looking, and `waiver.matches-nothing` reports each lapse as
`waiver-expired` (`rules/waiver.rs:62-75`).

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Leave it: file 19 waivers with a far `Expires` | No engine change | An expiry that means nothing; yearly churn; the cause stays prose |
| B. File 19 waivers with no `Expires` and a free-text `Reason` | Works today | Ungoverned (items 1-4 above); nothing stops the same shape being used to silence any rule |
| C. Declared evidence-free value in the record's own `Realized-by` | No separate file; zero engine change if a tier is declared | See below |
| D. A per-type exempt status | Visible in the record | A status the engine must treat as done; `ADR-84` option E rejected a "partial" status on the same ground; every consumer of the success set learns it |
| E. A single waiver scoped to many records (list or glob) | Fewer files | See below |
| F. **Declared permanence on the suppression type, with a reason vocabulary (chosen)** | Opt-in; general; machine-readable cause; reuses the existing waiver, `effective_requires` and stale-waiver machinery; keeps the exception a separate reviewed record | A new config key and one new waiver field |

**Why not C.** `embodiment.tiers` is declared config (`.urzua/config.yaml:11-14`) and `judge_delivery` only
asks whether the computed tier is in `unevidenced_tiers` (`status.rs:182`), so a repository could declare an
extra tier (`{state: Exempt, evidence: exempt}`) and write `exempt:<path>` in the record. This is the strongest
alternative and the cheapest. It loses on review: the exemption is written by the same hand, in the same file,
as the claim it excuses, and `ADR-11` exists to make an exception a separate reviewable record. The category is
a path, not a vocabulary value, so nothing can count or restrict causes. The tier applies to every type that
declares `statuses.delivery`, so a milestone could use it to read as delivered. And no stale check applies: an
exempt record that later gains a test stays exempt silently. (Whether a declared tier with no `claims_built`
behaves as described should be verified by the implementing change before relying on this paragraph.)

**Why not E.** `Scope` is one exact path or `*` (`waiver.rs:89`). A list needs a second field, and a glob is a
string mini-language that `ADR-64` (structured in, structured out) steers away from. More important, one
waiver covering 19 records breaks the stale rule: `waiver.matches-nothing` asks whether `any` finding is
covered (`rules/waiver.rs:110`), so 18 stale entries hide behind one live one. Per-entry reporting would have
to be built, and each record's cause could not be stated in its own `Reason`. One waiver per record gives 19
small files, each reviewable and each reported on its own.

## Decision

In the context of exceptions that are permanent facts about a record rather than debts, facing a waiver
mechanism that allows permanence but cannot govern it, we decided that **the suppression type may declare, per
rule, that a waiver without `Expires` is allowed, and which structured reasons allow it**, to achieve a
permanent exception whose cause is machine-readable, counted and checked, accepting a new config key and one
new waiver field, and that 19 records get 19 waivers.

- **Declared on the suppression type, beside `effective_requires`** (`config.rs:665`), as an opt-in key.
  Illustratively:

  ```yaml
  permanent:
    - rule: status.delivery-agreement
      reason_field: Category          # a field the type declares
      categories:
        - {name: docs-only}
        - {name: performance-only}
        - {name: test-removed-by-decision, cite: required}
      cite_field: Justified-by        # a pointer field the type declares
  ```

  `rule` is a rule id this build ships; `reason_field` and `cite_field` must be fields the type declares
  (validated at load the way `effective_requires` is, `config.rs:1597-1625`); `categories` is a non-empty,
  duplicate-free list; `cite` is `required` or omitted. The engine names no category, rule or field. The
  vocabulary above is this repository's, not the engine's.
- **Effect.** When `permanent` is declared, a waiver with no `Expires` takes effect only if its `Rule` has an
  entry, its `reason_field` holds one of that entry's categories, and its `cite_field` resolves to a record when
  the category requires one. Otherwise it is **ineffective**: it suppresses nothing and its finding is reported
  again, through the `unmet` path (`waiver.rs:58`, `is_effective` at `waiver.rs:81`, `apply_suppressions` at
  `waiver.rs:150-155`) and reported as `waiver-ineffective` by `waiver.matches-nothing` (`rules/waiver.rs:76-90`),
  naming the missing category or citation. A waiver **with** `Expires` is unchanged. This fails toward more
  findings, as every waiver failure does (`waiver.rs:93-99`).
- **Undeclared: behaviour unchanged.** A repository that declares no `permanent` key keeps today's behaviour,
  in which a no-expiry waiver is allowed for any rule. (That is the existing behaviour, not a restriction
  this ADR adds; the maintainer's request assumed an expiry was always required, and it is not.) Declaring the
  key is also how a repository closes the default: after it, permanence exists only for the listed rules.
- **Honesty.** A permanent waiver stays honest by the mechanisms that already apply to every waiver, plus one:
  (1) it is still listed in the report as `waived`, never omitted (`waiver.rs:147-150`); (2) the stale rule
  applies without regard to expiry: when the record gains a test or is fixed so the finding stops, the waiver
  matches nothing and is reported (`rules/waiver.rs:104-121`), so a permanent waiver cannot outlive its cause;
  (3) a rule renamed or removed is reported as unknown (`rules/waiver.rs:91-103`); (4) a citation is a pointer
  field, so `pointer.resolution` reports a cited record that is later retired or superseded; (5) new: the
  report counts permanent waivers by category, so the number and the causes are visible on every run (the
  population line or a notice; the milestone fixes the form).
- **Scope stays one record per waiver.** `Scope` is unchanged. A list or glob scope is not added (option E).
- **Rule, not variant.** `permanent` is keyed on the rule id, the only thing a waiver matches (`waiver.rs:85`).
  A finding's variant (`ActionSource.variant`, `report.rs:163`) is not serialized or matchable. Narrowing to a
  variant (`delivery-without-evidence` only) is deferred; see open question 2.
- **Not a candidate.** `BUG-72`, `BUG-90` and `BUG-146` are unevidenced for no permanent reason and get no
  waiver here. They stay findings until evidence exists or the maintainer decides otherwise.

## Reversibility

Moderate. The key and the waiver field are additive and opt-in, and a repository declaring nothing is
unchanged. The waivers filed under it are ordinary records. The cost sits in the vocabulary: once records carry
a category, renaming one is a mass edit, and a category that is too broad becomes a way around `ADR-84`. The
declared vocabulary is config, so it can be tightened by a one-line change and the affected waivers then
report as ineffective.

## Consequences

- `SPEC-3` documents `permanent`; `SPEC-10` documents the field, the effect and the count; `SPEC-2` gains the
  `waiver-ineffective` wording for the new causes. Each gets a revision row. `SPEC-10`'s "not built" list is
  unaffected.
- This repository declares `permanent` for `status.delivery-agreement` and files up to 19 waivers
  (the 24 less `BUG-142`/`147`, which cite their tests, and `BUG-72`/`90`/`146`). The 5 "covered by existing
  suites" are a weak fit: a covering suite is evidence, so it belongs in `Realized-by` as a `test:` locator,
  not in a waiver. They should be cited, not waived, unless the maintainer decides a refactor is a cause in its
  own right (open question 3).
- Once `permanent` is declared, a no-expiry waiver for any other rule stops taking effect. Existing waivers all
  carry `Expires`, so none changes.
- A waiver gains a second required field when its category requires a citation, and the `waiver` template
  gains `Category` and `Justified-by`.
- The permanent set is now a count in the report, which makes growth in it visible in review.

## Open questions

1. **Close the default?** Should declaring `permanent` make a no-expiry waiver ineffective for every rule not
   listed (chosen above), or only add the vocabulary for listed rules and leave other rules as today?
   Recommended: close it; a repository that declares `permanent` is saying where permanence is allowed.
2. **Variant scoping.** Should an entry be able to name a variant? Recommended: not now; the stale rule
   already reports the waiver once a record gains evidence, and variant matching needs the variant carried into
   the finding contract.
3. **Is "covered by an existing suite" a category?** Recommended: no; cite the suite in `Realized-by`. If a
   refactor has no single covering test, the category can be added to this repository's vocabulary without an
   engine change.
4. **Is the citation required for any category?** Recommended: only where the cause is another decision
   (`test-removed-by-decision` citing `RFC-35`); the vocabulary stays small.
5. **Re-examination on change.** Should a permanent waiver be reported when its scoped record is edited after
   the waiver (a `Reviewed-on` comparison)? Recommended: no in the first build. The stale rule fires on the
   event that matters, a finding disappearing, and a date comparison adds a field to maintain.

## References

- `ADR-84` -- the rule that needs the exemption route; `ADR-11` -- the waiver as a record; `SPEC-10` -- its
  schema; `MILE-168` -- the stale-waiver rule; `BUG-177` -- `effective_requires`.
- `ADR-64` -- structured in, structured out; `ADR-53` -- governance is declared config.
- The open PR building ADR-84 for the defect type, and its review. Milestone: `MILE-193`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-05 | Drafted as Proposed. **Why:** maintainer request of 2026-11-05: waivers do not let a repository state a permanent exception on a declared reason, which `ADR-84`'s exemption route needs for 19 `Fixed` defects with no test to cite. | **substantive** |
