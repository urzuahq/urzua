# Agent instructions

This file is for any AI agent working in this repository. `CONTRIBUTING.md` is the human-facing
equivalent; read both, but this one states things CONTRIBUTING.md doesn't need to.

## Use the tool on itself

This project's own backlog is tracked through `urzua`'s own record types, not scratch notes or
conversation history. If you're doing multi-step work worth remembering:

- **Track it as a `milestone`** (`urzua new milestone "..."`) if it's planned work — set `Phase`,
  `Track`, and `Implements` (comma-separated, pointing at whichever RFC/ADR it realizes) honestly.
  A milestone with no `Implements` yet is valid if the decision doesn't exist yet either.
- **Track it as a `bug`** (`urzua new bug "..."`) if you find a real defect — even one you fix in
  the same turn. `Regression-test` is required: name the specific test that proves it, or the test
  you plan to write. Cross-link the milestone that plans the fix and the bug that describes the
  defect via a real `Implements` pointer, not just a prose mention — verify the edge actually
  resolves with `urzua graph` before treating the record as done.
- Before assuming something isn't already tracked, run `urzua check docs/`, `urzua explain <path>`,
  or `urzua graph` against the real corpus rather than trusting memory of an earlier turn.

## Verify before trusting

This project's own history is full of defects found by actually running the tool against real
input, not by reading the code and assuming it does what its comment says. Before treating a claim
about behavior as true:

- Run the actual command against a real or disposable fixture. `check <path>` looking identical to
  `check docs/` was found this way — nothing about reading the code would have surfaced it.
- A new rule or fix needs a **planted-violation test, observed failing before the fix, then passing
  after** — not just a test that always passed. If you can't show it failing on the old code, it
  isn't verified yet.
- Don't assume a design decision matches what's actually implemented. Grep the code, don't just
  read the ADR.

## Never silently rewrite an Accepted decision

If understanding changes after an ADR/RFC is `Accepted`, don't edit its Decision/Consequences
content to match. Either add a dated, transparent note to its own Consequences section explaining
the correction, or write a new record that narrows/supersedes it and say so explicitly in both
places. `Status` is never silently changed during an otherwise-structural edit.

## A record's Status is a human decision, not yours to set

Filing an RFC, bug, or milestone is not the same as deciding it. More than once in this project's
history an agent set an RFC's `Status` to `Rejected` on its own initiative, without being asked to
— the author had to catch it and ask "did I say to reject it?" each time. Create records as
`Draft`/`Open`/`Planned` and leave `Status` there. Only move it to `Accepted`/`Rejected`/`Fixed`/
`Done`/etc. when the user explicitly says to. This is distinct from the rule above: that one is
about not silently editing an *already-decided* record's content; this one is about not deciding it
in the first place.

## Don't build speculative capability

Ship the smallest thing that solves a real, evidenced case. This project has walked back more than
one over-eager build this way — a general `migrate header-shape` command, a hard cutover to a
single header format — after checking whether real demand existed and finding none yet. When a
bigger version of something is plausible but unproven, build the narrow version and name the bigger
one as a milestone, don't build it preemptively.

The same instinct shows up as backward-compatibility code written for adopters that don't exist
yet. Legacy pre-type-prefix filename parsing (`ADR-36`) was kept "indefinitely" for a mixed corpus
that turned out never to materialize — checked live, zero real files ever used the old shape, and
the dead branches sat unaudited for a full revision because nothing re-checked the prediction that
justified them. This tool has no real external adopters today; don't add a compatibility shim,
tolerance, or silent default "for when someone needs it." If a real case shows up, build for it
then.

## Don't patch around a finding on your own new work

A live finding on something you just wrote is signal, not a bug in the checker. If your own new
record or field trips a rule (an undeclared field, a dangling reference), the honest options are:
make the underlying change the rule is actually asking for, or leave the finding firing as a true,
visible fact while the real decision gets made separately. Do not widen a `known_fields` list, add
an exception, or otherwise adjust config just so your own diff comes up clean — that's the same
shim instinct as the backward-compatibility case above, aimed at your own output instead of an
imagined adopter. Also watch for repeating an anti-pattern you just finished removing elsewhere in
the same corpus (e.g. writing a `Field: TARGET (Status)` annotation right after shipping a rule that
flags exactly that shape) — check your own new writes against any rule you just built or fixed.

## Before calling anything done

- `make ci` passes locally (fmt, clippy, build, test, `urzua check docs/`).
- A changeset exists in `.changeset/` for anything a person installing `urzua` would care about
  (ADR-29) — skip only for CI config, internal refactors, or docs-only changes.
