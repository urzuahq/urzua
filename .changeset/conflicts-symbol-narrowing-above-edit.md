---
default: patch
closes: [BUG-212]
---

`urzua fix --conflicts` no longer refuses a record where one branch narrowed a whole-file pin to a symbol (`path#symbol`) and the other branch edited the same file above that symbol and re-pinned the whole file: the symbol is pinned over the merged file. The earlier rule that an edit above a pinned scope refuses the merge now applies to line ranges only (and to a range or symbol both branches re-pinned). The refusals for a range now say the other branch edited or moved lines above it, and a merge that combined both branches' changes across the edge of a range or symbol they both re-pinned says the merged span does not end where the branch's does. Splitting one whole-file pin into two scopes at one site is refused and needs a manual re-pin.
