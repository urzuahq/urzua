---
default: minor
closes: []
---

New command `urzua describe` prints one JSON object that says how to call urzua: every command with its full-path name, profile, whether it is safe to run, what it reads on standard input, and each argument's kind, type, whether it is required or repeatable, its meaning, accepted values and the arguments it requires or conflicts with; `--config` and `--as-of` are declared on the commands that take them; also the environment variables urzua reads and its configuration file. It follows the `describe` shape of the published agent CLI contract and declares contract version `1.0`. Twenty-one arguments that had no help text now have one in `--help`, and `migrate schema` refuses a missing `--report` or `--field` at parse time, naming it.
