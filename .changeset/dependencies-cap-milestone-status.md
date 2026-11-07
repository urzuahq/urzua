---
default: patch
closes: []
---

A record whose Done means items all resolve but which depends on an unfinished record now computes `InProgress`, not `Done`, so `status.delivery-agreement` and `sequence.status-agreement` agree on one status for it. `sequence.status-agreement` skips that record with an info notice, `all items proven ahead of dependency X`, and still reports a record in progress out of order, one with an unresolved item. The finding, and the `urzua fix` notice, name the dependency that caps it. Configurations that declare no Done means items or no sequencing pair are unchanged.
