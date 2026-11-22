---
default: minor
closes: []
---

A symbol locator can tell a field from a method of the same name. Each `member` entry of a `locator_resolvers` resolver may declare a `kind` (any name you choose, such as `fn` or `field`), and a locator selects one with a suffix: `src/cfg.rs#Cfg::known[fn]` or `src/cfg.rs#Cfg::known[field]`. Only the entries of that kind are tried. A suffix naming a kind no entry declares is reported by `embodiment.locator-exists` with the kinds the resolver does declare, and a malformed suffix (`Cfg::known[]`, `Cfg[fn]`) is reported as malformed. A malformed `kind`, or two entries of one resolver with the same kind, is a config load error naming the resolver. Locators without a suffix resolve exactly as before, so declaring kinds stales no existing pin.
