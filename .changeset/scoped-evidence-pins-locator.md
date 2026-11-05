---
default: minor
---

A `Realized-by` locator can carry a scope between its path and its pin: `code:path#Symbol` for a
named item, `code:path#Type::member` for a method found across all of a type's `impl` blocks,
`code:path#Type::<Trait>::member` for a method of a trait impl, or `code:path#L10-L40` for a line
range. Symbols are found by resolvers a repository declares in the new top-level
`locator_resolvers` (a glob, a start pattern, and an end kind: `brace_balanced`, `indent`,
`heading`, `blank_line` or `semicolon`); the engine knows no language. `embodiment.locator-exists`
reports a locator that does not parse, a scope that does not resolve (not found, ambiguous, no
resolver, end not found, range past the end of the file, file not UTF-8), a scope given without a
pin, and a commit anchor, which is reserved and not yet supported.
