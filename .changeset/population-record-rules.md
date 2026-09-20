---
default: patch
---

The five record-scoped rules report a population. `revision-log.change-class-required` now shows
`eligible: 307, examined: 236` on this repository — 71 records carry no revision-log marker, which
`BUG-50` records as indistinguishable from compliance. The report performs that subtraction for the
first time. `header.layout-consistency` reads `eligible: 0`, which is a rule with nothing in scope
rather than a rule whose matcher is broken.
