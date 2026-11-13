---
default: minor
closes: [MILE-187]
---

A fact that nobody builds can now be a dependency. This repository declares a `condition` record type
(`COND-NNNN-slug.md`, statuses `Open`, `Met` and `Dropped`, with `Met-by` required once `Met`) entirely in
`.urzua/config.yaml`, and a `Depends-on` from a milestone or any other record that declares the field
resolves to one like any other record. `urzua roadmap` and the derived `blocked` value read it: a
dependent of an `Open` condition is blocked and out of the ready set, a dependent of a `Met` one is
not, and a dependent of a `Dropped` one stays out. Any repository can declare the same shape under its
own type name; the engine has no special case for the name. `SPEC-23` documents the record.
