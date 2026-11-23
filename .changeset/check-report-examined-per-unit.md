---
default: minor
closes: []
---

`urzua check`'s JSON report gains `examined`, an object with one count per unit (`{"files": N}`), so a gate that reads per-unit counts needs no adapter. It sits beside `files_examined`, which stays and always equals `examined.files`; no key is renamed or removed. A `not-run` report that could not start (it carries `error`) prints neither key; one that started and selected nothing prints both at 0.
