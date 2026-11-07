---
default: patch
closes: []
---

`check` is organised as separate stages for loading, running rules, scoping and deciding the verdict, with the
verdict a pure function. Output is unchanged for every run.
