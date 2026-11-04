---
default: minor
---

A record type can own more than one directory. Declare `extra_dirs` beside `dir` and every listed
directory is part of the type's one identity namespace: `urzua new` numbers past the highest id in
any of them (and past a number claimed there on another branch), pointers and citations resolve
across them, and `identity.collision` and `filename.identity-pattern` see them as one population.
A directory listed twice, or owned by two types, is a config error, and an `extra_dirs` entry that
does not exist is reported by `type.dir-matches-nothing`.
