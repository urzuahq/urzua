---
default: minor
closes: []
---

`urzua explain --types` prints, as JSON, what the configuration declares for every record type, and `urzua explain --type <name>` for one: its directories, prefix, fields with their declared values, what each reference field may name, its states with the initial ones and the allowed moves, each `statuses.require` and `statuses.forbid` entry with its condition keys and one generated sentence, any authority gate with who may act, the template in use and the `urzua new` command that creates one. It also lists the rules the configuration sets, the rules on because of a declaration, and the rules that would report a declaration but are off. Every entry carries the key path and line it was read from; the only defaults printed (an undeclared initial state, a rule on by declaration) are marked `"default": true`. An undeclared type name exits 2. The harness's session-start report names the command.
