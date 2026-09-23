---
default: patch
---

`urzua init` swallowed a hypothetical YAML serialization failure into an empty config body, reporting
`written: true` with no error -- if it ever fired, an adopter would get a config with no
`record_types`/`rules` and no sign anything had gone wrong. Changed to fail loudly instead, matching the
same "should structurally never fail" precedent already used for JSON output elsewhere. No input this
function is called with today can actually trigger the failure; this closes the failure mode itself.
