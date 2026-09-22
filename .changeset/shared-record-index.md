---
default: patch
---

`pointer.resolution`, `pointer.target-status`, `narrative-field.stale`, `claim.status-agreement` and
`relation.supersession-reciprocity` each rebuilt their own normalized id-to-record index independently
on every `check`/`audit` run — the same O(corpus) work done five times over. `check` and `audit` now
build one shared index per run and pass it to each rule. Verified as a no-op: findings, `files_examined`
and `records_read_by_any_rule` are unchanged on this corpus. `relation.supersession-reciprocity`'s own
index used to resolve a duplicate-id collision last-wins; the shared index resolves it first-wins
instead, which changes nothing observable since `identity.collision` already reports any such collision
on its own.
