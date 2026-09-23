---
default: minor
---

Adds a new opt-in rule, `config.known-fields-declaration-missing`: reports a record type that
declares no `known_fields` at all, so a repository can require every type to make its field-set
governance an explicit choice (`known_fields: []` at minimum) instead of leaving it as the unchecked
default `header.field-set-consistency` gives an undeclared type. Mirrors
`config.pointer-declaration-missing`'s shape and message style exactly.

Design decided in `RFC-43`/`ADR-62`, in response to a code-review finding that correctly identified a
real (but already-decided, `ADR-53`-governed) gap: an undeclared field, including a case-variant of an
already-declared one, goes unchecked for a type with no `known_fields`. This rule gives a repository a
lever to close that gap for itself without changing the lenient default for everyone.

Enabled in this repository's own config in the same change; no fixes were needed since all six
declared record types already declare `known_fields`. No adopter-facing behavior change for a config
that doesn't enable the new rule: verified with the full test suite, clippy, `make ci`, and a
real-corpus `check` run reporting the same 70 findings before and after.
