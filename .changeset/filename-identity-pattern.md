---
default: minor
---

Add the opt-in `filename.identity-pattern` rule. A file directly in a record type's directory whose name does not match the type's declared `identity.pattern` (or the `TYPE-NNNN-slug.md` convention when none is declared) is now reported instead of silently dropping out of every identity-reading rule. When every file of a type is off-pattern, as in a bare-number corpus (`0001-slug.md`), `check` and `audit` report it once for the type with the expected file name for each file, so the corpus can be renamed. `urzua init` proposes the rule at `warn`.
