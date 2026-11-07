---
default: patch
closes: []
---

The corpus acceptance report no longer counts a rule that merely ran as a rule that can fire. It
states, per rule, whether the suite has a failing case, a clean run only, or no case, and the
suite gains failing cases for twenty-three rules that had one of the last two. Acceptance scratch
repositories are named by corpus as well as case, so two corpora that share a case id no longer
share a directory. Integration tests run the binary with a stub `gh`, so no test depends on the
machine's login or the network. `make ci` now runs `check --base` and the knope pin check, and
says which CI checks it leaves out. The release workflows install knope through one action, the
release App token is requested with only the permissions the job uses, and the merge queue runs
the diff rules. A release-invariants guard now reads a fragment's level by key rather than from
line two.
