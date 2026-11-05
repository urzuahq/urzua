---
default: minor
---

`diff.source-requires-record` can now be a decision gate. Declare `decision_types` (each record type that holds decisions, with the statuses at which one counts as decided), and optionally `fix_types` (record types, such as `bug`, whose records exempt a change) and `link_fields` (pointer fields, such as a milestone's `Implements`, through which a record links a decision). A diff that changes declared source paths must then also change a decision at a decided status, a record linking one, or a fix record; otherwise each changed source path is a finding, with an `add-record` suggested action, and a changed decision still at an undecided status is named in the message. The rule is unchanged unless `decision_types` is declared, and a declaration that could never apply (an unknown type, a status the type does not list, `fix_types` without `decision_types`) is refused at load.
