---
default: patch
---

A record type can declare pairs of fields that must be declared together with `companion_fields`, a
list of relation role pairs such as `[embodiment_state, embodiment_locator]`. The new
`config.companion-declaration-missing` rule reports a type that declares `Embodiment` without
`Realized-by`, or the reverse, which left records able to claim a state no rule could check.
