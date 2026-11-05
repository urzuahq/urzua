---
default: minor
---

A scoped evidence pin drifts only when the hash of the item it names changes: an edit elsewhere in
the file is not drift, and a scoped pin has no file-based grace window (a declared audit date still
holds it back). `urzua explain <path>#<scope>` resolves a scope and prints the span, its hash and
the locator to paste into `Realized-by`. `urzua fix --rebaseline` shows the resolved span beside the
hashes, re-pins a scoped pin in the spelling it was written, and lists a pin whose scope no longer
resolves under `failed_pins` instead of re-pinning it.
