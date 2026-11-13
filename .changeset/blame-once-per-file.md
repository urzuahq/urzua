---
default: patch
closes: []
---

`check`, `fix` and `reverify` run `git blame` once per record file instead of once per line, and ask git for each
commit's time once, so a run over a large corpus starts several times fewer git processes and finishes sooner. Output is
unchanged for every run.
