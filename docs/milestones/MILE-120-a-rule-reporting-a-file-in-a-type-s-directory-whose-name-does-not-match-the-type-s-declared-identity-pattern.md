---
Stable-Id: 01M6CXZVVVP6PK91GPCXN3SWSK
Status: Planned
Phase: '0'
Track: schema-governance
---
# 120 — A rule reporting a file in a type's directory whose name does not match the type's declared identity pattern

## What

A rule that reports a file in a record type's declared directory whose name does not match that type's
declared identity pattern. Today such a file is not a record of the type: it is silently absent from
every rule's population, and nothing says so.

## Why

A first real adopter corpus of several hundred records using bare zero-padded numeric filenames
(`0173-slug.md`) is exactly the case where a handful of files named off-pattern (a draft, a typo, a
renamed template) would vanish from every check while still sitting in the directory. `SPEC-1`'s
no-silent-no-op rule applies: a file the directory declares but the type cannot read is a finding, not
an absence. This is the file-level sibling of `type.record-outside-declared-dir`, which covers the
reverse direction. Needed for `MILE-102`.

Since `RFC-46` was rejected, renaming is the only supported route for a corpus in another filename shape, so this rule is also how the engine says so: when every file in a type's directory is off-pattern (a whole bare-number corpus), `urzua init` and `urzua audit` report the mismatch once per type, with the expected `TYPE-NNNN-slug.md` name for each file, instead of silently running the identity rules on zero records. That covers the bare-filename case of `BUG-99`, and the acceptance suite's bare-filename known gap becomes an assertion of this guidance.

## References

- `type.record-outside-declared-dir` -- the existing reverse-direction rule.
- `MILE-98` -- the declared identity pattern this rule compares filenames against.
- `RFC-46` -- rejected; renaming replaces tolerating bare-number filenames.
- `MILE-134` -- the later rename tool.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed. **Why:** An adopter's off-pattern filename in a type's directory is invisible to every rule; filed as a first-adopter gap. | **substantive** |
> | 2026-11-02 | Scope widened to whole-corpus mismatch guidance in `init` and `audit`. **Why:** `RFC-46` was rejected, so the engine must explain the one convention rather than tolerate others. | **substantive** |
