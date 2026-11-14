---
default: patch
closes: []
---

The output contract now holds on stdout alone. `check`'s `blocking` is true exactly when the run
exits 1, so under `fail_on: warning` an unwaived warning reads `"blocking": true` (it read `false`
beside exit 1). A path argument that selects no record gets a warning notice even when another
argument matches, and every `not-run` report says why in an `error` or a notice. `waive` reports
the identity it used in place of `--by`, and numbering's notices, in a `notices` field, and `new`
reports a missing type directory as a notice. `rules` carries `"status": "ok"`. In the MCP server a
`not-run` report is an error result, and `urzua_list_records` no longer carries the whole-corpus
`roadmap` block.
