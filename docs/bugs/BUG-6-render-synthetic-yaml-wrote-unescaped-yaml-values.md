---
Stable-Id: 01M1ZK63KBP2T4QK7HVET3M1KJ
Status: Fixed
Found-in: found during ADR-33's yaml-frontmatter migration work, before writing the actual conversion tool -- reviewing render_synthetic_yaml's own output shape ahead of reusing its escaping logic
Realized-by: test:rust/crates/urzua-core/src/new_record.rs#render_synthetic_yaml_escapes_a_value_with_an_embedded_colon@sha256:ae49b0810382bf0438cb971cf5455534159646f42f2a8ca784c6056b7ce1275b, test:rust/crates/urzua-core/src/new_record.rs#render_synthetic_yaml_round_trips_a_numeric_looking_string@sha256:2260e991b1883bccfb880ab6689728a8547e09baccce94a7938885802f0e63d6
---
# 6 — render_synthetic_yaml wrote unescaped YAML values

## What was wrong

`render_synthetic_yaml` (`new_record.rs`, what `urzua new` uses for any `yaml-frontmatter`-shaped
type) built its output with raw `format!("{field}: {value}\n")` string formatting -- zero YAML
escaping. Harmless while every value it ever wrote was blank, but a real correctness bug the moment
a real value (a colon, a leading `-`, a value that merely looks numeric like `"0.2"`) needed writing:
a colon in the value would silently produce a second, bogus key on reparse; a numeric-looking string
would round-trip back as a YAML number, not the string it started as.

## Why nothing caught it

The function had never actually been exercised with a real value -- every type using the
`yaml-frontmatter` path so far (`adr`/`rfc`/`spec`, pre-migration) only reached this path when no
template existed, and none of `required_fields`'s blank placeholders needed escaping. Found live
while auditing `render_synthetic_yaml` ahead of reusing its logic for the corpus migration tool
(ADR-33/MILE-3), not from a failing test -- nothing exercised a real value through this path before
that audit.

## Fix

Rewrote `render_synthetic_yaml` to build a real `yaml_serde::Mapping` and serialize it with
`yaml_serde::to_string`, instead of hand-formatting strings -- the real serializer's plain-scalar
analysis quotes a value exactly when leaving it bare would change what it parses back as. Verified
against the actual parser (`header::parse_with_shape`), not by inspecting the output string: a value
with an embedded colon (the same shape as MILE-4's real `Blocked-on` field, since
`header::parse_key_value` only splits on the first colon) and a numeric-looking string both round-trip
to their exact original value.

A second, related gap found live while fixing this one: the function also unconditionally emitted
`Date`/`Author` regardless of whether a type's `required_fields` actually named them -- fine for
`adr`/`rfc`/`spec`/`waiver` (which all name at least one), but `milestone`/`bug` name neither, so the
very first yaml-frontmatter `milestone` created this way (MILE-85) carried both fields unannounced,
tripping `header.field-set-consistency`. Fixed in the same change: `Date`/`Author` are now only
filled with their real value when `required_fields` names them; `Stable-Id` alone stays universal
(ADR-21).

## References

- ADR-33 -- the migration this bug was found auditing ahead of.
- MILE-3 -- the corpus migration this fix unblocked (the migration tool reuses the same escaping,
  built fresh rather than copying this function's pre-fix bug forward).
- MILE-4 -- the real `Blocked-on` colon-value case the regression test is drawn from.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-08 | Initial bug record, `Status: Fixed`. | **structural** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-07 | Re-baselined (attested by beauwilliams, identity: gh-verified, actor: agent): re-pinned `rust/crates/urzua-core/src/new_record.rs#render_synthetic_yaml_escapes_a_value_with_an_embedded_colon` (lines 704-733) 15315798b5a0 -> ae49b0810382, `rust/crates/urzua-core/src/new_record.rs#render_synthetic_yaml_round_trips_a_numeric_looking_string` (lines 735-759) beb7ebeb72b1 -> 2260e991b188. Reason: The two pinned tests build NewRecordParams literals, which gained the fields title_field and initial_version (both None here) for the declared title and starting version of urzua new (MILE-74). Their inputs and assertions are unchanged. **Why:** the pinned evidence changed and an agent re-verified that the claim still holds, to be confirmed by a reviewer in the PR. | **structural** |
