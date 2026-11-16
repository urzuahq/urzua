---
default: minor
closes: []
---

Three evidence guards. **An upgrade can surface findings.** On `urzua check --base` the new rule
`diff.pin-changed-unattested`, on by default at `error`, requires every pin of a record the base holds to be
unchanged or carry the row a pinning tool writes, so a pin edited, added, moved or removed by hand now fails.
The command for each case: a stale pin, `urzua fix --rebaseline --apply`; a first pin, the new
`urzua fix --pin --apply`; a new scope or a renamed file, `urzua fix --pin --apply --replace OLD=NEW`; a dropped
pinned locator, `urzua fix --pin --apply --remove OLD` (each with `--ids <record> --actor agent --reason "<...>"`).
A renamed record is judged against its old copy, pins are compared occurrence by occurrence, and a pull request
that turns the rule off is still judged at the base revision's level. `fix --pin` works on any record, with no
expiry period. An agent's `urzua fix --apply` on a path-gated record is now refused: `fix --apply`,
`fix --rebaseline --apply`, `fix --conflicts --apply`, `fix --pin --apply` and `reverify --apply` write a record
under a path gate only for `--actor human` under a `gh`-verified login in the gate's group (`--actor` is a claim:
this rests on that login), and `waive --apply`, `index --apply`, `migrate ids --apply`, `migrate filenames --apply`
and `new` refuse under a path gate, however the path is spelled. A required suppression field, or an override
log's `Reviewed-by` cell, with no letter or digit (`—`, `-`, `…`, an emoji, a link with no text) is blank, so
such a waiver no longer takes effect.
