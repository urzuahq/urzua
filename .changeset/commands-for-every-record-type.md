---
default: minor
closes: [MILE-199]
---

`urzua harness install` now writes a drafting command for every record type the configuration declares (for example `/bug`, `/condition`, `/waiver` and `/spec` beside `/adr`), generated from `.urzua/config.yaml` with only the type name and directory filled in, and three lifecycle commands: `/amend` adds a dated amendment section and a revision-log row to a decided record and refuses any change to existing lines; `/repin` runs `urzua fix --rebaseline --actor agent` only with a non-empty reason and prints the line for the pull request body; `/done` records finished work by running `urzua fix --apply`, so the state written is the one the engine computes from the evidence and the command never writes a state itself. Adding a record type to the configuration and running `urzua harness update` adds its command, and `urzua harness status` reports a command not yet installed as missing. A record type named like a lifecycle command is refused at install. `urzua fix --apply` no longer writes a state that an `authority` gate names; it reports the refusal.
