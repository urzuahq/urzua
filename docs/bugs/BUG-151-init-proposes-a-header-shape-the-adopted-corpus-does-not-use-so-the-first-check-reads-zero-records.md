---
Stable-Id: 01M6CXZZ2TP2VC2SZ8CJWVTKZM
Status: Open
Found-in: "The first check of a real adopter corpus (bold-list headers, bare zero-padded numeric filenames) after urzua init"
Regression-test: "planned: an init test adopting a bold-list fixture asserts the proposed header_shape is bold-list"
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

## References

- `ADR-33` -- deprecates bold-list, converges on yaml-frontmatter; adopt mode proposes the latter.
- `BUG-73` -- `init` proposes rules whose declarations it did not write.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** a first adopter corpus read as zero records under the config `init` wrote. Left `Status: Open`: the fix needs a decision on `ADR-33`'s steer, not only a code change. | **substantive** |
