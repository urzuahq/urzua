---
default: patch
closes: []
---

Hardens `urzua reverify` (`ADR-97`). A locator that points outside the repository or at a file git does not track no longer resolves for a pin, in `reverify`, `fix --rebaseline` and the drift check. A first pin is written on the exact locator, so pinning `a.rs#Foo` no longer edits `a.rs#Foo::bar`. At an `error`-level period any actor but `--actor human` is refused, including an omitted `--actor`. `--force --ids` renews the named record. Detect lists a claim whose record has no usable revision log as refused, with the reason, instead of offering a renewal that `--apply` refuses. `apply_reverify` and `apply_rebaseline` share their evidence-line and revision-row code, with no change to what either writes.
