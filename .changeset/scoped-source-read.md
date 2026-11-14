---
default: patch
closes: []
---

Make `urzua check <path>` read only the source files the back-pointer rules need. A run with a path
argument and no `--base` reads the files under the requested paths, and any other file whose
`Implements:` or `Verifies:` comment cites a record in scope, for `embodiment.back-pointer-resolution`,
`embodiment.back-pointer-decision-accepted` and `code.dangling-bug-reference`, instead of every file
under `back_pointer_paths`. On a benchmark repository of 2,361 tracked files a single-record run went from 4.61 s to
1.91 s; on a small repository the run takes about 59% of a whole-corpus run's time, and the saving grows
with the number of source files, so it is much faster on large repositories. Its
findings are a whole-corpus run's restricted to those paths, plus the decision findings about the records
in scope; the files it did not judge are counted in each rule's population as `deferred_by_scope`, with
an `info` notice. A run with no path argument and a `--base` run read every file, as before.
