---
default: minor
closes: [MILE-201]
---

Adds `attestation`, a top-level configuration key that sets where `urzua fix --apply` and `urzua fix --rebaseline --apply` take the identity they attest under. `identity: gh-verified`, the default and what a configuration without the key gets, lets a verified `gh` login win over `--by`, as before, and the report now names the `--by` it replaced in an `identity` field. With `require_verified: true` the command exits 2 with `not-run` and writes nothing when `gh` cannot verify a login. `identity: asserted` takes `--by` as given and records the verified login beside it; combining it with `require_verified` is refused. An unknown `identity` value is a load error naming the allowed ones.

Adds `--actor agent|human` to `urzua fix`. Every tool-authored revision row now records the identity source (`gh-verified`, `asserted` or `git-config`) and the actor kind (`agent`, `human`, or `unspecified` when the flag is absent), for example `Re-baselined (attested by NAME, identity: gh-verified, actor: agent)`. The actor kind is the caller's claim, not proof. Existing rows are untouched.
