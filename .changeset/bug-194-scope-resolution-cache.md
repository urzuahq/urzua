---
default: patch
closes: [BUG-194]
---

`check`, `audit`, `fix` and `reverify` resolve each `#symbol` scope in a locator once per run and compile each resolver
pattern once, instead of recompiling every pattern and scanning the file twice per locator. On a repository with many
scoped pins a run takes about half the CPU time. Output is unchanged for every run.
