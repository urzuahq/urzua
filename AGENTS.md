# Agent instructions

This file is for any AI agent working in this repository. `CONTRIBUTING.md` is the human-facing
equivalent; read both, but this one states things CONTRIBUTING.md doesn't need to.

## What urzua is, and what it refuses to be

`urzua` is a governance engine for decision records (ADRs/RFCs/specs/milestones/bugs/waivers as
data, not prose convention), built on one founding claim: **one config-driven implementation
instead of every team hand-writing its own linter.** SPEC-1 states the constraints that follow from
that claim; read it before assuming a change is obviously fine.

- **Config-driven is the whole thesis, not a nice-to-have.** The same binary has to serve
  differently-shaped repos without forking. Anything that only works because of a hardcoded,
  type-specific assumption in Rust source (a field name, a record type, a relationship) is a
  regression against the founding claim, not a shortcut — this is what BUG-8/RFC-23/ADR-44 exist to
  fix, and the same question is worth asking of any new rule.
- **No silent no-op.** A check that finds nothing must be distinguishable from a check that ran on
  nothing — report what was examined, not just what was found. This is SPEC-1's own acceptance-test
  bug class, not a style preference.
- **Three distinct field states, not one.** Blank, placeholder, and pending are different things;
  collapsing them was the single most-repeated bug class across the tools this project replaced.
  Watch for the same collapse in any new field-state logic.
- **Structural presence is not content-scope correctness, permanently.** A required section can
  exist, have the right shape, and pass every check while still misrepresenting the actual decision
  — `check` cannot and must not claim to close that gap mechanically. Never design a rule as if it
  could verify a section's *meaning*, only its *presence and shape*.
- **The real bar is replacement, not coexistence.** SPEC-1's own success criteria: a tool that runs
  *alongside* the hand-written linter it was meant to replace has failed, whatever its test coverage
  says. Weigh new work against whether it moves toward an adopter deleting their own linter, not
  just toward more checks existing.

## Git workflow

Never commit directly to `main`. Create a feature branch and open a PR for every change, however
small — including a one-line docs fix. Check `git branch --show-current` before your first edit if
there's any doubt which branch you're on.

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
- **`urzua new`'s assigned display number is provisional, not reserved.** It's a scan of the local
  filesystem, blind to numbers already claimed by other open, unmerged branches or PRs (MILE-89) — a
  real collision this project has hit live. Before opening a PR, check whether another open PR
  claims the same number for the same type; a collision found this way is expected and cheap to
  rename, not a bug in the tool.
- **Before hand-writing any record header or field, check `.urzua/config.toml`'s
  `header_shape`/`known_fields`/`pointer_fields`/`narrative_fields` for that type.** Declared config
  is the source of truth for header shape and field vocabulary — never freehand an annotation
  convention (e.g. baking a status or explanation into a pointer field's own value) because it looks
  consistent with nearby records; check what's actually declared, and check it again against any
  rule you've just built or fixed in the same session, since matching an anti-pattern you just
  banned elsewhere is a real, observed failure mode.

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

Filing a record is not deciding it. Create records as `Draft`/`Open`/`Planned` and leave `Status`
there — only the user moves it to `Accepted`/`Rejected`/`Fixed`/`Done`. The same rule covers any
other field whose value asserts a decision was reached on someone's behalf — `Supersedes /
Superseded-by`, a vacated-number tombstone, closing a cross-reference — not just the literal
`Status` key itself. Distinct from the rule above: that one guards an already-decided record's
content; this one guards the decision itself.

## Don't build speculative capability

Ship the smallest thing that solves a real, evidenced case. This project has walked back more than
one over-eager build this way — after checking whether real demand existed and finding none yet.
When a bigger version of something is plausible but unproven, build the narrow version and name the
bigger one as a milestone, don't build it preemptively. The same applies to backward-compatibility
code: don't add a shim, tolerance, or default "for when someone needs it" without a real case in
front of you — this tool has no external adopters yet whose behavior needs preserving.

If you find *existing* compatibility or tolerance code whose stated justification no longer holds
against this repo's own real, current state — a "for future adopters" comment where no adopter
exists, a "for a mixed corpus" rationale where the corpus already converged — that's a live finding
worth its own bug record, the same as any other defect. Don't leave it un-flagged just because you
didn't write it, and don't silently delete it either without a decision on the record.

## Don't patch around a finding on your own new work

A live finding is signal, not a bug in the checker — whether it's on code you just wrote, code
you're merely touching, or unrelated code you happened to notice. Fix the real gap it names, or
leave it firing while the decision gets made separately — don't widen a config list, an enum, or an
ignore-list just to make a diff come up clean, regardless of whose diff it is or how old the
underlying issue is.

## Before calling anything done

- `make ci` passes locally (fmt, clippy, build, test, `urzua check docs/`).
- A changeset exists in `.changeset/` for anything a person installing `urzua` would care about
  (ADR-29) — skip only for CI config, internal refactors, or docs-only changes.
