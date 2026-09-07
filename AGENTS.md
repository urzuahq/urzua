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

## Don't build speculative capability

Ship the smallest thing that solves a real, evidenced case. This project has walked back more than
one over-eager build this way — a general `migrate header-shape` command, a hard cutover to a
single header format — after checking whether real demand existed and finding none yet. When a
bigger version of something is plausible but unproven, build the narrow version and name the bigger
one as a milestone, don't build it preemptively.

## Before calling anything done

- `make ci` passes locally (fmt, clippy, build, test, `urzua check docs/`).
- A changeset exists in `.changeset/` for anything a person installing `urzua` would care about
  (ADR-29) — skip only for CI config, internal refactors, or docs-only changes.
