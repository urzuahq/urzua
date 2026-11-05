---
default: patch
closes: []
---

`urzua check --base` now agrees with a full check in five more cases. Changing or deleting a waiver
reports the finding it stopped waiving on the unchanged record. A comma-joined pointer value such as
`ADR-1,ADR-3` traces each reference when one is deleted. A changed file below a declared directory no
type owns keeps its `type.record-outside-declared-dir` finding. Retargeting a symlinked configuration
puts every record in scope. An empty diff with no rule enabled is `not-run` (exit 2), as a full run is.
