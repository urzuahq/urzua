---
default: patch
---

A path argument and `--base` now keep or drop each finding by what its rule reports on, declared once
on the rule, instead of by a list of rule ids kept in `check`. No finding is kept or dropped
differently.
