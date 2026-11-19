---
default: patch
closes: [BUG-209]
---

`urzua fix --conflicts --ids <record>` now fails when the entry matches no unmerged record, including in a tree where git lists nothing unmerged. Before, that run reported `nothing-conflicted` and exited 0 with the entry only in `failed`. It now reports `partial-failure`, exits 1 and names `urzua fix --conflicts` as the next action. `fix --conflicts` with no `--ids` and nothing unmerged is unchanged.
