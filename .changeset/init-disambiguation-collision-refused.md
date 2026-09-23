---
default: patch
---

`urzua init` disambiguates a proposed type name when two directories share the same last path
component (`docs/adr` and `legacy/adr` would both propose `adr`), rewriting the colliding ones to
their full, hyphen-joined path. The collision check itself only ever compared base names, never the
disambiguated name it produces -- so a disambiguated name could still collide with an unrelated
directory that already happened to be named that, and the generated config's `Mapping::insert`
silently overwrote one type's entry with the other's. An entire directory's records could disappear
from `.urzua/config.yaml` on the very first `init` run, with no error and a config that still loaded
and passed cleanly.

`init` now checks the fully-disambiguated names for a second collision and refuses (exit 2) rather
than silently dropping one, naming both colliding directories and the shared name.
