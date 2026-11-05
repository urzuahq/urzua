---
default: patch
---

`urzua fix --apply` no longer corrupts a record whose `Embodiment` is `null`, `~` or empty. It used to
write the computed value at the start of the line (`ImplementedEmbodiment: null`) and report success;
it now fills the value in place, and the revision row reads "stated no value" instead of an empty span.
