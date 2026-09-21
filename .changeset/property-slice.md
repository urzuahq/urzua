---
default: patch
---

Adds the first slice of `SPEC-4`'s acceptance suite (`MILE-101`): a seeded, hand-rolled generator over
an alphabet of characters that have broken case-insensitive comparisons, and two properties checked
against it. The declared-field matcher is extracted as `rules::field_is_declared`/`fold_field_name`, so
the suite tests the matcher the rules use rather than a restatement of it. No rule behaviour changes.
