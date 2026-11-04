---
default: minor
---

Add `types` to a `relation_pairs` entry: `types: [rfc]` binds the pair to the record types that carry it. `relation.pair-reciprocity` then treats a claim to a type outside the list as out of scope, counted in the report, instead of an unechoed claim from a type that cannot hold the inverse field. A pair without `types` stays global. An unknown or duplicated type, an empty list, or a type declaring neither of the pair's fields is refused at load.
