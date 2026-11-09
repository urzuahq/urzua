---
default: minor
closes: []
---

Add `authority.waivable` (`never`, `base` or `any`) to decide whether a waiver record can silence
`diff.decider-required`. Under `base` only a waiver the base revision already holds counts, and under
`never` none does, so a pull request cannot accept a record by adding a waiver in the same change.
`urzua init` proposes `never` and `urzua doctor` warns when gates are declared and the key is not.
`doctor` also warns when no gate lists `.urzua/config.yaml`. Under `decider_table: base` or `both`, the
rule's `require_evidence` option is now the stricter of the base's and the pull request's, and a plain
local `urzua check` no longer prints the "no --base" decider-table notice.
