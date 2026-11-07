---
default: patch
closes: []
---

Paths the tool reads or writes on a name from the arguments or the configuration are confined to the repository, symlinks included. `index --apply`, `fix --apply`, `migrate ids --apply`, `refs refresh` and `init` refuse to write through a symlink that leaves the repository, and `explain` refuses an absolute path, a `..` path or a symlink to a file outside it. `urzua --config FILE init` writes `FILE` instead of `.urzua/config.yaml`. `fix --reason` refuses a carriage return, `fix --rebaseline --apply` hashes the evidence after taking the lock, and the not-implemented message names the right spec.
