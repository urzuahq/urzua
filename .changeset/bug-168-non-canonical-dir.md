---
default: patch
---

A record type's `dir` or `extra_dirs` entry written as `./docs/adr`, `docs/adr/` or `docs//adr` is now a
configuration error that names the canonical spelling (`docs/adr`). It used to load, match nothing and run
as `not-run` with no reason.
