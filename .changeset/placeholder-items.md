---
default: minor
closes: []
---

Adds `pending` to `statuses.delivery.items`, an optional list of item texts that mean a Done means item is not yet written, in the shape of `statuses.pending` (a whole value or `{pattern: "..."}`). An item that matches is an unfilled placeholder: it never resolves, cannot be waived, and `status.done-items-shape` reports it on its line, so a milestone scaffolded from a template no longer passes `urzua check` with its placeholder items unflagged. A configuration without `pending` behaves as before.
