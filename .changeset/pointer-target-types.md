---
default: minor
closes: []
---

A reference field can declare the record types it may name. A `pointer_fields` entry may be written `{name: Parent, targets: [epic]}` instead of a bare name, and a relation pair may declare `target_types` for its forward field. The new rule `pointer.target-type`, on by default at `error`, reports a reference that resolves to a record of another type, naming the field, the target, its type and the allowed types; each element of a list is judged, and a reference that names no record is still `pointer.resolution`'s alone. A target type that is not a configured record type, an empty list, or a `cross_type: false` pair listing a type that does not carry its forward field refuses to load. `urzua graph` now lists the edges of every declared relation pair. A configuration that declares no target types behaves as before.
