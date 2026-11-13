---
default: minor
closes: []
---

`urzua fix --stdin-path <path>` reads the proposed content of one record from stdin and judges it
without writing, for an editor hook that wants a verdict before the file is saved. The path need not
exist yet, and drift is computed for that record only, so the run takes a fraction of a whole-corpus
`urzua fix`. Each repair now also carries `line`, the 1-based line of the stated field, so a hook can
point at it without parsing the record.
