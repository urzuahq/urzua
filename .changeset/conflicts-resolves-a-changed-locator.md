---
default: patch
closes: [BUG-210]
---

`urzua fix --conflicts` now resolves a record where one branch replaced a pinned locator (narrowed `code:path` to `code:path#symbol` or a line range, or moved it to another file) and the other branch only re-pinned the old one. It used to refuse these when the locator was in a Done means item, and keep the narrowing side's hash unchecked when it was in `Realized-by`. The changed locator wins in both places, its pin is recomputed from the merged file, and the merge revision row names the locator it replaced. A locator both branches replaced with different ones is still refused, naming both.
