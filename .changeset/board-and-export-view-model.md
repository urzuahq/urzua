---
default: minor
closes: []
---

`urzua board` writes one self-contained HTML page of every record, laid out only from what the configuration declares, and prints a JSON answer naming the file; `urzua board --serve [--port N]` answers the same page at 127.0.0.1 (GET only, re-reading the records on each request, `Cache-Control: no-store`) until interrupted. The page opens with what waits on a person, what is under way and what is ready, then the read stamp (commit, branch, uncommitted record changes, time), then work as the tree the configuration declares, the other types as compact rows, and finished and dropped records collapsed under their count. Nothing is left off.

`urzua export --format json` gains, additively, what a view of the records needs: each record's `title`, a `fields` map, `status`, `state` (finished, dropped, under-way, not-started or none) with the declaration that decided it, `waiting_on`, `ready`, `gated_moves`, `parent`, `children`, `children_by_state` and `state_disagrees_with_children`, `refers_to`, `unresolved_refs` and `referred_by`, and a `link`; each section's `ticked` and `unticked` counts and, for a declared checklist, its `items`; and, at the top, `type_roles` and a `read` stamp. Records are in numeric order within a type. `--no-raw-text` leaves the text out. The read-only server's record tools return the same keys.

New declarations: `part_of: true` on a `pointer_fields` entry that declares `targets` makes that reference containment (a configuration without it has no tree), and an optional `export.link_prefix` makes each record carry a `link`. `urzua roadmap` grouping by a field now follows a record to its nearest ancestor's value when the record carries none, instead of returning `null` under every child.
