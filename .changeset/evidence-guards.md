---
default: minor
closes: []
---

Three evidence guards. **An upgrade can surface findings.** A pin edited by hand now fails
`urzua check --base`: the new rule `diff.pin-changed-unattested`, on by default at `error`, reports a
record whose `@sha256` pin changed (or was removed) since the merge base when the record gained no row
written by `urzua fix --rebaseline --apply`, `urzua reverify --apply` or `urzua fix --conflicts --apply`
for that exact change; a configuration sets another level or `off` under `rules:`, and `off` raises a
warning notice. An agent's `urzua fix --apply` on a path-gated record is now refused: `fix --apply`,
`fix --rebaseline --apply`, `fix --conflicts --apply` and `reverify --apply` write a record under a path
gate of `authority` only for `--actor human` under a `gh`-verified login in the gate's group, and
`waive --apply`, `index --apply`, `migrate ids --apply`, `migrate filenames --apply` and `new` refuse
under a path gate; each refusal names who can make the change. A required suppression field, or an
override log's `Reviewed-by` cell, with no letter or digit (`—`, `-`, `…`, an emoji) is blank, so such a
waiver no longer takes effect.
