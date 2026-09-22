---
Stable-Id: 01M33QRYRW15K91N5S6HXF41C0
Status: Accepted
Date: 2026-09-22
Author: beauwilliams
---
# 40 — a declared field lookup that misses needs one contract, not one guess per rule

## Summary

`ADR-57` decided a declared field *name* compares exactly. It never decided what a rule does when
that exact-cased lookup misses — and two rules answered differently, in opposite directions, both
silently. This proposes one shared helper for reading a declared field's value, with one declared
outcome for a miss, so a tenth rule reading a declared field cannot invent an eleventh answer.

## Motivation

`Header::get(key)` (`header.rs:43-48`) is `find(|f| f.key == key)` — correct per `ADR-57`, and the
only sanctioned way to read adopter-declared vocabulary (`Header::get_reserved` exists precisely to
keep the engine's own reserved keys, `Stable-Id`/`Rule`/`Scope`/`Expires`, out of this path). But
`ADR-57` specified the comparison, not the caller's obligation on a miss, and two rules that both read
a declared field through `get` chose incompatible obligations:

**`claim_status_agreement` (`rules.rs:1483-1550`) fails open.** A miss on `Status` becomes a sentinel:

```rust
let status = target.header.get("Status").unwrap_or("(no Status field)");
if closed_statuses.iter().any(|s| s == status) {
    continue;
}
```

The sentinel never matches a configured closed status, so the `if` never fires and every occurrence
falls through to a pushed `Finding` at `Error` severity. A record whose author wrote `status:` instead
of `Status:` — legal YAML, one keystroke — turns every claim that cites it into a blocking error, with
a message ("has Status (no Status field)") that reads as a status problem rather than a lookup miss.
`BUG-100` reproduces this.

**The embodiment family (`rules.rs:2106`, `~2185`, `~2244`) fails closed, silently.** Eligibility is
decided from config alone — `declared_slots` (`~2062-2077`) checks whether the type's
`required_fields`/`known_fields` contain the literal string `"Realized-by"`/`"Embodiment"` and never
opens the record's header. The rule body then does:

```rust
let Some(value) = record.header.get("Realized-by") else { return Outcome::Absent; };
```

A cased-differently key produces the same `Outcome::Absent` as a type that legitimately never adopted
the field. There is no `Finding`, and the only trace is `eligible > examined` in the JSON report,
which nothing reads as a defect — it is indistinguishable from the cold-start case the four-state
`Outcome` model exists to represent honestly. `BUG-101` reproduces this.

**Both are the same root cause reached in opposite directions.** Neither rule decided this on
purpose; each just wrote the obvious code at its own call site, and "the obvious code" was a different
guess each time. A rule written tomorrow reading a third declared field gets to guess a third way, and
nothing will say so — the same shape `RFC-39` describes for comparison, one layer up: `RFC-39` is
about how two values are compared once found; this is about what happens when one of them is not
found at all. `RFC-39` and this proposal share evidence (`ADR-57`'s exact-match) but not a fix — a
`FieldName` newtype makes the comparison rule structural and says nothing about the miss case this
document is about.

## Proposal

**Every rule reading a declared field's *value* — as opposed to checking whether a slot is
declared, which `declared_slots` already does correctly from config — goes through one function that
returns a three-state result, not `Option<&str>`:**

```rust
pub enum FieldRead<'a> {
    Present(&'a str),
    /// The header parsed; this exact key was not among its fields.
    Missing,
    /// The header did not parse at all; no field could be read.
    Unreadable,
}

pub fn read_declared_field<'a>(record: &'a Record, key: &str) -> FieldRead<'a>
```

A caller pattern-matches; there is no default to reach for by typing `.unwrap_or(...)`, which is what
let `claim_status_agreement` invent a sentinel in the first place. `Missing` and `Unreadable` are kept
distinct because they are already distinct everywhere else in this codebase (`Outcome::Absent` vs
`Outcome::Unreadable` in the population model) — a parse failure and a spelling miss are different
facts about the record and a reader benefits from knowing which.

**The declared obligation on a non-`Present` result: the candidate is `Absent`, and no `Finding` is
pushed *about that comparison*.** This is Decision 1 from the population-honesty plan, applied to
field reads instead of field slots: *do not emit a finding about a candidate you did not examine.*
Concretely:

- `claim_status_agreement` changes its `unwrap_or` to a `continue` on `Missing`/`Unreadable` — the
  claim is not judged, not judged-and-wrong. `header.required-fields` already exists to report a
  missing declared field on its own, once, correctly attributed; `claim_status_agreement` duplicating
  that report as a fabricated status mismatch is `BUG-100`'s whole shape.
- The embodiment family keeps returning `Outcome::Absent` on a miss — that part was already right —
  but the *population* must stop conflating "never declared" with "declared and unreadable". Today
  `declared_slots` computes `eligible` from config alone, so both land in the same bucket. This
  proposal's `eligible` still comes from config (a record either has the type's `required_fields`
  containing the key or it does not — that part is not in question), but `examined`'s complement
  needs a way to say *this candidate's slot was declared, present in `required_fields`, and the header
  simply didn't have that exact key* — which is a `MILE-106`-shaped signal (a declared policy the
  engine could not apply), not silence. The precise mechanism (a new `Outcome` variant vs. surfacing
  it through `MILE-106`'s existing `config.scope-matches-nothing` rule) is an open question below.

**A lint, not just a convention.** A convention that "declared field values are read through
`read_declared_field`, never `header.get(...).unwrap_or(...)`" is exactly the kind of rule this
project's own history says does not hold without a check (`RFC-39`'s six independently-wrong
comparison sites, this proposal's own two). Add a `purity.rs`-style scan (the existing dependency
scan's sibling) forbidding `.header.get(` outside `header.rs`, `read_declared_field`'s own
implementation, and the reserved-key call sites already routed through `get_reserved`. A planted
violation — a new call site written as `header.get("X").unwrap_or(...)` — must be caught failing
before this lands.

## Open questions

- **Does the embodiment family's fix need a new `Outcome` variant, or does `MILE-106`'s
  `config.scope-matches-nothing` already cover it once wired to field-level populations?**
  `MILE-106` currently judges *rule*-level populations (`PopulationUnit::Rule`); extending it to
  field-level `eligible > 0 && examined == 0` gaps is a smaller change than a fifth `Outcome` state,
  but was explicitly scoped out of the original population-honesty plan ("Do not build `MILE-106`'s
  rule here"). This RFC does not resolve which; both keep the same external contract (a case-mismatched
  declared field becomes visible, not silent).
- **Should `read_declared_field` subsume `Header::get` entirely**, or coexist with it for the small
  number of sites that already correctly treat a miss as legitimate absence (e.g. an optional
  `known_fields` entry a rule is allowed to skip without comment)? Coexisting risks a fourth call
  shape; subsuming means auditing every existing `.header.get(` call site once, not just the two this
  RFC found by review.
- **Is a compile-time lint (a `purity.rs` scan run in `make ci`) sufficient, or does this want a
  clippy lint** so an editor flags it before commit rather than before push? The existing
  dependency-allowlist scan sets the precedent for the cheaper option.

## Non-goals

- **Does not reopen `ADR-57`.** Field names still compare exactly; this is entirely about what happens
  after that comparison fails to find a match.
- **Does not implement `RFC-39`'s newtypes.** `FieldName`/`RecordId`/`StatusValue` are a separate,
  independently useful change; this RFC's `read_declared_field` can be written against `&str` today
  and re-typed later without changing its contract.
- **Does not fix every `.header.get(` call site in one pass.** Migration order and full-site audit are
  left to the implementing milestone.

## References

- `ADR-57` — field names compare exactly; this RFC's whole motivation is a gap in what that decision
  specified.
- `RFC-39` — the sibling proposal for comparison-site drift; shares evidence, not a fix.
- `BUG-100`, `BUG-101` — the two call sites that motivated this, in opposite failure directions.
- The population-honesty plan (`MILE-101`/`SPEC-4`'s design work) — Decision 1 ("do not emit a
  finding about a candidate you did not examine") is reused here, applied to field reads rather than
  field slots.
- `MILE-106` — the existing declared-but-unreachable-policy rule this proposal may extend rather than
  duplicate.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Filed. **Why:** a `/code-review v0.3.0...main` pass found `claim_status_agreement` and the embodiment rule family answering "the declared field's exact-cased key is missing" in opposite, undeclared ways (`BUG-100`, `BUG-101`). Patching each call site separately would add a ninth answer to a question that already has two contradictory ones; this proposes the one the project is missing. | **substantive** |
> | 2026-09-22 | `Status: Draft` → `Accepted`; decided by `ADR-58`. **Why:** the proposal's scope was small enough to decide and implement in one pass rather than leave open across a release. `ADR-58` rejected a fourth `FieldRead` state (a differently-cased miss carrying its own value) in favor of one dedicated rule, `header.field-case-mismatch`, diagnosing it -- narrower than this RFC's own open question, which left that choice unresolved. | **substantive** |
