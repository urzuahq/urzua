---
Stable-Id: 01M6CXZZ2TP2VC2SZ8CJWVTKZM
Status: Fixed
Found-in: "The first check of a real adopter corpus (bold-list headers, bare zero-padded numeric filenames) after urzua init"
Realized-by: test:rust/crates/urzua-cli/tests/check_integration.rs#init_on_a_bold_list_corpus_proposes_yaml_frontmatter_and_names_the_removed_form@sha256:7e086e70e86bbff5664ebced45a82e4ad8ee19c333f74e040c2bb3750200b7e3
---
# 151 — init proposes a header shape the adopted corpus does not use, so the first check reads zero records

## What was wrong

`rust/crates/urzua-cli/src/commands/init.rs` (line 143) inserts `header_shape: yaml-frontmatter` for
every proposed record type, whatever shape the corpus on disk actually uses. On a real adopter corpus
of several hundred records with bold-list headers, the first `check` using the config `init` wrote
emits one `header.required-fields` warning per record and reports `records_read_by_any_rule: 0`: the
tool read nothing and said so only as hundreds of identical warnings.

## Why nothing caught it

`init`'s tests use fixtures whose headers already are yaml-frontmatter, so the proposed shape and the
corpus's shape always agreed. No test adopts a corpus in a different shape and checks that the result
is readable.

## Notes

`ADR-33` decided `init` proposes `yaml-frontmatter` for newly adopted types, on the argument that adopt
mode recommends the destination shape. The fix may need a decision on how `init` reconciles that steer
with what a corpus actually uses: proposing the shape the corpus has and flagging it deprecated, or
proposing the destination shape and reporting that the existing records need migrating. Either way the
config `init` writes must not silently read zero records. `BUG-73` is the nearest record (`init`
proposes rules that examine zero records) and has the same observable symptom for a different cause.

## Fix

The maintainer decided the policy: in adopt mode `init` proposes the header shape each type's records
actually use, so the first `check` reads the corpus. For every proposed type `init` now runs the real
header parser (`header::parse_with_shape`, no new parser) over the type's records under each supported
shape and proposes the one they parse under. A type whose records disagree gets the majority shape plus
an `init` notice naming the split; a type whose records parse under no shape keeps `yaml-frontmatter`
with a notice. A greenfield or empty type keeps `yaml-frontmatter`. The deprecation steer is carried by
`header.deprecated-shape`, which `init` already proposes at `warn` for every config it writes; `ADR-33`
records the narrowed decision as a dated amendment and `SPEC-5` describes the new behavior.

One limit on the regression test, found while writing it: `records_read_by_any_rule` is 0 under the
config `init` writes even for a correctly-shaped yaml-frontmatter corpus, because `init` proposes
`required_fields: []` and no `known_fields`, so no rule has a declared field to read. The count cannot
distinguish the fixed from the broken config, so the test asserts the real symptom instead: the first
`check` emits no `no header-shaped region found` warning (three of them before the fix, none after) and
`header.deprecated-shape` fires. Whether `init` should propose `known_fields` so that the count means
something on a fresh adopt is a separate decision.

## References

- `ADR-33` -- deprecates bold-list, converges on yaml-frontmatter; adopt mode proposes the latter.
- `BUG-73` -- `init` proposes rules whose declarations it did not write.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** a first adopter corpus read as zero records under the config `init` wrote. Left `Status: Open`: the fix needs a decision on `ADR-33`'s steer, not only a code change. | **substantive** |
> | 2026-11-02 | `Status: Fixed`; `Regression-test` now names the real test. **Why:** `init` proposes the header shape each type's records use (maintainer decision, `ADR-33` amended), fixed in the same PR as the decision. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 1 `test:` locator and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
