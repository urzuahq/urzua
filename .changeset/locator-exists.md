---
default: minor
---

New rule `embodiment.locator-exists`: a `Realized-by` locator naming a path that
is not in the working tree (`BUG-49`).

Nothing checked this. `embodiment.consistency` asks whether a locator's content
changed since the claim was written, so a locator naming nothing has no history
to compare and drifts past the one rule built to notice. The whole `Embodiment`
model rests on these paths -- `Verified` and `Implemented` are computed from
them -- so a record could claim verified work while naming a deleted file, and
the claim read as stronger than `Not started` rather than weaker.

Found live: one of this project's own records had named a file deleted two days
earlier, in a corpus the tool gates on every push.

Opt-in and levelled like every rule. The CLI supplies path existence, so
`urzua-core` stays pure.
