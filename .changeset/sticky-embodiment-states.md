---
default: minor
closes: []
---

`embodiment.consistency` takes an optional `sticky_states` list of `Embodiment` values a person sets
that no evidence can compute, such as `Deprecated` and `Inactive`. A record stating one is no longer
reported as drift or as an old claim, and `urzua fix` leaves it alone. A listed value that evidence
does compute is refused at load, since it would hide real drift.
