---
default: minor
---

`field.quality` takes the optional `template_placeholders` option. Declared `true`, a required field whose value is
identical (after trimming) to its record type's template text for that field, from `.urzua/templates/<type>.md`, is
reported as a placeholder, so a record left with `Found-in: how/where this was actually discovered` no longer passes.
The compiled-in placeholder tokens do not apply while it is declared, and a type with no template is judged by its
declared `pending` vocabulary alone. The option is off unless declared, and declaring it with no template for any
record type is reported as a notice.
