---
default: patch
---

Adds a mechanical guard (`crates/urzua-cli/tests/relation_field_literals.rs`) against
`BUG-118`/`BUG-119`/`BUG-120`'s recurring shape: a fix to `RFC-42`/`ADR-61`'s declared relation-field
names landed wherever a reviewed diff touched, while a sibling call site elsewhere in the tree kept
reading the old literal, three times in one review round. The new test walks every source file in
`urzua-core`/`urzua-cli` and fails if `"Status"`, `"Embodiment"`, `"Realized-by"`, or
`"Supersedes / Superseded-by"` reappears as a bare string literal in production code outside
`config.rs`, where `RelationRole`'s own defaults are declared. Ships with planted-violation cases,
verified to actually fail against the real shape of all three bugs before being restored.
