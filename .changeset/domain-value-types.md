---
default: patch
---

Implements `RFC-39`/`ADR-59`: `FieldName` and `RecordId` are now real types whose `Eq`/`Hash`
encode their comparison rule (exact for field names per `ADR-57`, numeric-normalized for record
identifiers per `BUG-2`), replacing bare `String` at the six field-name comparison sites and the
shared record index. `RecordTypeConfig`'s own fields stay `Vec<String>` -- the type boundary sits at
`declared_fields()` and the header's comparison methods, not at config deserialization, since that is
where every comparison bug in this family actually lived. The remaining ten rule functions that took a
bare `HashMap` projection of configuration now take `&Config` directly, matching the fifteen that
already did, removing the class of defect where two identically-typed but differently-scoped
projections could be silently swapped at a call site. Two enums that predate `ADR-53`'s move of status
vocabulary to the adopter (`urzua_core::Status`, `urzua_core::Embodiment`) are removed; neither was ever
constructed. No adopter-facing behavior changes: verified with the full test suite and a real-corpus
`check` run reporting the same findings before and after.
