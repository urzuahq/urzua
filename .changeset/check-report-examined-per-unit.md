---
default: minor
closes: [MILE-358]
---

`urzua check`'s JSON report gains `examined`, an object with one count per unit (`{"files": N}`), so a gate that reads per-unit counts needs no adapter. It sits beside `files_examined`, which stays and always equals `examined.files`; no key is renamed or removed. A `not-run` report prints neither key.
