---
default: minor
---

A symbol resolver can now address a struct field. The `member` of a `locator_resolvers` entry may be a list of entries, each with its own `start` pattern and `end` kind, and a name is looked for with all of them, counted together (a field and a method of one name are reported as ambiguous). Declare a method entry (`brace_balanced`) and a field entry (`indent`), let the `container` match a `struct` as well as an `impl`, and `Type::field` pins one field, so a new field elsewhere in the type no longer stales the pin. A single `member` mapping is read as before.
