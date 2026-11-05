---
default: patch
closes: [BUG-74]
---

The record type that suppresses findings is now declared with `role: suppression` in `.urzua/config.yaml`, so a
corpus can call it anything. Only one type may declare it. A config that declares no such type keeps using the
type named `waiver`, so existing configs behave as before.
