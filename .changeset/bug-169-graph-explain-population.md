---
default: patch
---

`urzua graph` and `urzua explain` now report a `population` (`eligible`, `examined`, `unreadable` and
`out_of_scope` records, as `check` does for a rule), and a warning notice lists any record left out because
its header could not be read. Status stays `ok`, so an empty result from an unread corpus is no longer
indistinguishable from an empty result from a read one.
