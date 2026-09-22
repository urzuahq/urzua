---
Stable-Id: 01M33SVCRJNRDBFBQY0AZZQSEZ
Status: Accepted
Date: 2026-09-22
Author: beauwilliams
Deciders: beauwilliams
Derives-from: RFC-39
---
# 59 — domain values the adopter declares are types not bare strings

## Context

`RFC-39` documented two conflations, both `String`/bare-collection shaped: a field name's comparison
rule was rewritten correctly six times at six call sites because nothing stopped a seventh from picking
a seventh rule; and ten rule signatures took a bare `HashMap` projection of `Config` where fifteen
already took `&Config` directly, indistinguishable to the compiler, which is exactly how
`field.untrimmed-value` was wired to the wrong projection and silently skipped every type declaring
only `required_fields`.

Since `RFC-39` was filed, `RFC-40`/`ADR-58` closed a third instance of the same family — a declared
field's *value read* had no single answer for what a lookup miss means — by giving it one shared
function and one dedicated rule. That fix's own evidence (`BUG-100`, `BUG-101`) is further confirmation
of the pattern `RFC-39` named: an operation on the adopter's declared vocabulary, re-derived per call
site, drifts.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| Converge the ten rule signatures on `&Config`; leave `FieldName`/`RecordId`/`StatusValue` as `String` | Smallest change; closes the structural half `RFC-39` called "already proven" | Leaves the value half — the six-times-wrong-comparison evidence — open |
| Build all three newtypes plus the `&Config` convergence in one pass | Closes both halves `RFC-39` documented | `RecordId` touches every `normalize_id` call site (9, plus `graph.rs`) and the shared index's key type; larger surface in one pass |
| Build `FieldName` and `RecordId`; leave `StatusValue` as `String` with a recorded reason | Builds the two newtypes with a demonstrated defect history; treats the third on its own merits rather than by inertia with the first two | Asymmetric API: two domain values are typed, one isn't |

## Decision

In the context of `RFC-39`'s two conflations and a third instance (`RFC-40`) confirming the same
pattern, facing a choice between a narrow structural fix and the full value-typing proposal, **we
decided to build both `FieldName` and `RecordId` as newtypes, converge the ten straggler rule
signatures on `&Config`, and not build `StatusValue`**, to make a field-name or record-id comparison
impossible to get wrong without an explicit `.as_str()` a reviewer can see, accepting an API asymmetry
where two of the three originally-proposed types exist and one doesn't, because the case for it
weakened rather than held.

**`FieldName`** (`values.rs`) wraps `String`; derived `Eq`/`Hash` on the wrapped string *is* `ADR-57`'s
exact comparison, so no custom comparison logic is written — the type's value is entirely that a bare
`&str` can no longer be substituted at a comparison site without going through `.as_str()`, which is
what makes a loose comparison visible in review instead of silent. `HeaderField.key` becomes
`FieldName`, and so does `Header::get`/`get_reserved`/`read_declared`/`duplicate_keys` and
`field_is_declared` — the six sites `RFC-39`'s own evidence table named as having independently gotten
this comparison wrong.

`RecordTypeConfig`'s four field-name collections (`required_fields`, `known_fields`, `pointer_fields`,
`narrative_fields`) **stay `Vec<String>`.** Reconsidered from `RFC-39`'s original proposal: these fields
are never themselves a comparison site — they are config declarations, converted into the shapes rules
actually compare against by `declared_fields()`/`required_fields_by_type()`/`field_slots` (all
consolidated this same release, `BUG-105`/`BUG-106`), and it is *those* projections' output types that
become `FieldName`-typed, not the stored config. Typing the stored fields directly would touch every
`RecordTypeConfig` literal in the test suite — over ninety sites, almost all of them test fixtures that
have never been a comparison bug — for no additional safety: a `Vec<String>` field that is immediately
projected into a `HashSet<FieldName>` on every read is exactly as safe as a `Vec<FieldName>` field
would be, and deserialization stays simple. The type boundary is drawn at the comparison, which is
where `RFC-39`'s evidence actually lived.

**`RecordId`** (`values.rs`) wraps two strings, not one: `normalized` (comparison and hashing) and the
author's own spelling for display where a caller wants it. This directly answers `RFC-39`'s own open
question ("does `RecordId` display normalized or as-written?"): `identity.collision`'s finding message
is the only place a `RecordId` itself is formatted, and it already showed the normalized form (the map
key), so `Display` shows normalized and nothing about output changes. Every other message quoting "the
author's spelling" (`pointer.resolution`'s `"{reference} resolves..."`, etc.) formats the raw reference
string directly and is never routed through `RecordId` for display, so that concern doesn't apply to
this decision at all — it was a different string the whole time.

**`StatusValue` is not built.** `RFC-39`'s own open question asked whether it was worth it, since
statuses are compared against a config-declared list rather than looked up, which is a weaker failure
mode than `FieldName`'s or `RecordId`'s. Since filing, `field.untrimmed-value` (round 12) already closed
the concrete defect a `StatusValue` would have targeted — a status carrying whitespace silently failing
an exact comparison — by *disclosing* the untrimmed value rather than normalizing it away, and
`RFC-40`/`ADR-58` closed the miss-handling half. A `StatusValue` that trimmed on construction would
undo `field.untrimmed-value`'s entire premise (the split between formatting and meaning); one that
didn't trim would add a type with no comparison rule beyond what `String`'s `==` already provides. There
is no comparison bug left for it to prevent.

**The ten `&Config` convergence.** Fifteen rules already take `&Config` and project internally; the
remaining ten (`header_required_fields`, `header_layout_consistency`, `header_field_set_consistency`,
`pointer_resolution`, `pointer_target_status`, `field_pending`, `field_quality`,
`field_untrimmed_value`, `header_field_case_mismatch`, `config_pointer_field_not_known` — the last
already took `&Config`) converge the same way. This removes the class outright: a bare collection
derived from configuration no longer crosses these function boundaries, so it cannot be wired to the
wrong projection the way `field.untrimmed-value` once was.

**Two dead enums.** `urzua_core::Status` and `urzua_core::Embodiment`, declared in `lib.rs` and never
constructed anywhere in the workspace since `ADR-53` moved status vocabulary to the adopter's
declaration, are deleted in the same pass. They are the fossil of the moment the vocabulary changed
hands, and their continued existence beside two newtypes actually built for adopter vocabulary would
misdescribe which strings in this codebase are typed and why.

**Newtype form: hand-written, not a macro.** Two types, following `RFC-39`'s own reasoning: a macro
earns its keep at a larger count.

## Reversibility

`FieldName` and `RecordId` are additive at their construction sites and mechanical to unwind (both are
thin wrappers with an `as_str()` escape hatch used nowhere in this pass, so reverting means deleting the
wrapper and un-wrapping call sites, not rewriting comparison logic). The `&Config` convergence is
already de-risked: this project rewired five call sites onto `&Config` in the prior release round
(`round-14-duplication`) with a real-corpus no-op check as the verification pattern, reused here.
Deleting the two dead enums is irreversible only in the sense that reintroducing them would need new
evidence they should exist, which is the point.

## Consequences

A field name can no longer be compared loosely without an explicit, greppable `.as_str()` at the six
sites that read a header field or judge a declared name. `RecordTypeConfig`'s own fields are unaffected
— its test fixtures need no changes, since the type boundary sits at `declared_fields()` and its
siblings, not at config deserialization. `RecordIndex`'s key type changes from `String` to `RecordId`;
every `normalize_id(&x)` call site becomes `RecordId::new(&x)`, and `identity_collision`'s reported
string is unchanged (already normalized). `StatusValue`'s absence means a status value stays a bare
`&str` at its comparison site — a deliberate, argued position, not an oversight, and the next reviewer
who wonders why should read this record rather than assume it was missed.

## References

- `RFC-39` — the proposal this ADR decides.
- `RFC-40`, `ADR-58` — the third instance of the pattern that confirmed the decision.
- `BUG-100`, `BUG-101` — evidence cited by both this ADR and `ADR-58`.
- `ADR-57` — the field-name exact-match decision `FieldName`'s `Eq` makes structural.
- `BUG-2`, `ADR-36` — the record-id numeric-normalization decision `RecordId`'s `Eq` makes structural.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-22 | Decided. **Why:** `RFC-39` was filed as Draft with three open questions; this project's own subsequent work (`RFC-40`/`ADR-58`, and `field.untrimmed-value` before it) answered two of them with evidence rather than assertion, which is why `StatusValue` is decided against here rather than deferred again. | **substantive** |
> | 2026-09-22 | Narrowed `FieldName`'s scope before implementation: `RecordTypeConfig`'s stored fields stay `Vec<String>`; only the comparison sites and the projections consumed from them become `FieldName`-typed. **Why:** counting construction sites before writing code found over ninety `RecordTypeConfig` literals, nearly all test fixtures that were never a comparison bug; typing the stored field would touch all of them for no additional safety once `declared_fields()` already centralizes the projection. | **substantive** |
