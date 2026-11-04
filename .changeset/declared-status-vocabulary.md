---
default: minor
---

A record type can now declare its status vocabulary in `.urzua/config.yaml` under `statuses`: the allowed `values`, which are terminal `success` or `failure`, the fields a status `require`s or `forbid`s, and the type's own `pending` values. Three new opt-in rules report against it: `status.undeclared-value`, `status.required-field-missing` (for example `Deciders` on an `Accepted` record) and `status.forbidden-field-present`. `field.pending` uses a declared `pending` list, matched as whole values, instead of treating any value containing the word as pending; `narrative-field.stale` and `claim.status-agreement` use a declared type's terminal statuses in place of their hand-written lists. A type that declares nothing behaves as before.
