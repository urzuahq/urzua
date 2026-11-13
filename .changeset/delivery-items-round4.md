---
default: patch
closes: []
---

An item with a valid waiver no longer fails `embodiment.locator-exists` while its evidence does not
exist yet, as the waiver intends. Messages about the delivery items name the section the type declares
instead of "Done means". A type whose `delivered_tiers` names a tier outside `resolving_tiers` is
refused at load, naming both lists. A locator that resolves outside the repository is no longer
probed on disk.
