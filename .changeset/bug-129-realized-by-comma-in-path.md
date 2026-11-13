---
default: patch
closes: [BUG-129]
---

A comma in a `Realized-by` locator path no longer truncates the path. A comma now separates entries only
when it is followed by a category prefix (`spec:`, `code:`, `test:`); any other comma is part of the path,
so `code:src/weird,name.rs` names `src/weird,name.rs`.
