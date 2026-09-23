---
default: patch
---

Fixes `BUG-118`, `BUG-119`, `BUG-120`: `RFC-42`/`ADR-61` made `Status`/`Embodiment`/`Realized-by`/
`Supersedes / Superseded-by` adopter-declared per type for the seven rules `urzua check` runs, but three
other consumers of the same fields were left reading the old literals directly — `urzua fix`
(`detect_repairs`), `urzua explain`/`urzua graph` (`explain`, `graph`), and drift detection
(`compute_drifted_records`). A type declaring a `relation_fields` override got `check` correctly judging
it while these three commands silently disagreed. All three now resolve the same
`relation_field_name`/`RelationRole` accessor `check`'s rules use.

Also: `config.relation-field-not-known` is now enabled in this repository's own config (it was added to
`ALL_RULES` in the previous round but not turned on here, unlike its sibling
`config.pointer-field-not-known`); `RelationRole` gained an `ALL` constant and `RelationFields` a `get`
accessor so a role-enumerating loop can't silently miss a future 5th role; the two "declared alias not
known" rules now share one `Finding`-construction helper; and `SPEC-2`'s rule table is resynced from
twenty-one to its real twenty-nine rows (`BUG-52` remains open as the actual missing-mechanism gap this
manual sync doesn't close).

No adopter-facing behavior changes for a config that declares no `relation_fields`: verified with the
full test suite, clippy, `make ci`, and a real-corpus `check` run reporting the same 70 findings before
and after.
