---
default: minor
closes: []
---

`urzua waive --from-report` gains `--apply`, which writes the drafts it previews. Each draft is a
waiver with a pending `Reason` and no `Expires`, numbered once under the same lock `urzua new` takes,
so a second `--apply` writes nothing. `urzua check` afterwards reports each original finding again
beside a `waiver-ineffective` finding on its draft, until a person fills in a real `Reason` and an
`Expires`. A write error stops the run with exit 2 and an `error` field, and `drafts[].written` says
which drafts were written. The report also lists, as skipped, a finding that persists even though its
own draft would hide it, and a finding inside a waiver record.
