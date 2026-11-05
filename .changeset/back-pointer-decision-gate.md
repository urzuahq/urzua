---
default: minor
closes: []
---

New opt-in rule `embodiment.back-pointer-decision-accepted` reports a source `Implements: ADR-12`
comment whose cited record is not yet in an accepted status, so code cannot claim to implement a
decision that has not been made. Accepted means the cited type's declared `statuses.success`; a
type without one uses the rule's optional `accepted_statuses`, and with neither the status is
reported as unjudgeable. It takes `back_pointer_paths` like `embodiment.back-pointer-resolution`,
and on a `--base` run judges only the source files the diff changed.
