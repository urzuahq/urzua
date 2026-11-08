---
default: patch
closes: []
---

`urzua harness run edit` now applies the protected-path and acceptance gates to every path of an edit request that names
several paths, so an agent cannot write a protected file by listing it beside another. A request with several paths is
refused when `on_unjudged: deny` is set. Requests with one path are unchanged.
