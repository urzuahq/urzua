---
default: minor
closes: []
---

`statuses.require` now treats the values in a type's declared `statuses.pending` list (for example `Unknown`, `none recorded` or a dash) as missing, so an accepted record whose required field holds a placeholder is reported by `status.required-field-missing`. The list is the same one `field.pending` reads and is declared once; a type that declares none behaves as before.
