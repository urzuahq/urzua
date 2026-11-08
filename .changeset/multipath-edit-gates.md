---
default: patch
closes: []
---

`urzua harness run edit` now applies the protected-path and acceptance gates to every path of an edit request that names
several paths, so an agent cannot write a protected file by listing it beside another. With the editor-hook guardrail on,
an agent's request with several paths is also refused when a guardrail set to `on_unjudged: deny` covers one of the
paths, and under `on_unjudged: deny` as before. A person's request with several paths, and an agent's that nothing gates
under `on_unjudged: allow`, is judged as before. Requests with one path are unchanged.
