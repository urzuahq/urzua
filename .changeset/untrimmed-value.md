---
default: minor
---

Adds `field.untrimmed-value`, reporting a declared field whose value carries leading or trailing
whitespace. Every rule compares values exactly, so `Status: "Superseded   "` is not `Superseded` and a
status rule configured to report that status stays silent on it. Trimming inside those rules would
accept a value the author did not write; this reports the whitespace where the author can fix it,
which is the split `yamllint` makes between formatting and meaning.

Reachable only through `yaml-frontmatter`: the blockquote parser trims at parse time, and YAML trims
an unquoted scalar, so it takes a quoted value to carry the space through.

`narrative-field.stale` previously trimmed the target status where `pointer.target-status` and
`claim.status-agreement` did not. All three now compare exactly.

Also adds `release-guard` to the `Makefile`'s `.PHONY` list, where it was missing while its sibling
`release-invariants` was present.
