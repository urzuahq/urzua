---
default: minor
---

A record type can declare its lifecycle: `statuses.initial` names where a new record may start and
`statuses.transitions` maps each status to the statuses that may follow it. The new opt-in rule
`status.transition-allowed` compares a changed record's `Status` at the merge base with the change
under check (`urzua check --base`) and reports an edge the graph does not list, or a new record that
starts outside `initial`. `status.unreachable-state` reports a declared status that no path from
`initial` reaches. The `embodiment` declaration must now mark at least one tier `claims_built`.
