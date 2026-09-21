---
default: patch
---

Files RFC-38, which proposes folding `urzua audit` into `urzua check --rules` and stopping `fix`'s
detect mode from re-deriving a comparison `check` already performs. Records-only; no behaviour
changes in this release. BUG-99 is re-diagnosed and blocked on that RFC rather than patched.
