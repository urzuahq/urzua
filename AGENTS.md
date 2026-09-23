# Agent instructions

This file is for any AI agent working in this repository. `CONTRIBUTING.md` is the human-facing
equivalent; read both, but this one states things CONTRIBUTING.md doesn't need to.

`docs/specs/SPEC-19-repository-agent-guardrails.md` governs this file: what belongs in it and why,
and its revision log is where this file's own change history lives, since this prose carries none
itself. Any substantive edit here adds a matching entry there, in the same PR.

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

## The record-type workflow

Six record types, each governing its own directory and each defined by its own `spec` (`SPEC-16`
adr, `SPEC-17` rfc, `SPEC-18` spec, `SPEC-6` milestone, `SPEC-9` bug, `SPEC-10` waiver) — read the
relevant one before hand-writing a record of that type, rather than inferring its schema from nearby
examples. What each is *for*, not just its fields:

- **`rfc`** — a proposal, not yet a decision. Lives in `Draft`/`Discussion` until an `adr`
  (`Derives-from: RFC-N`) accepts or rejects it. An RFC's own `Status` is independent of whether a
  governing ADR exists yet.
- **`adr`** — a point-in-time decision fact. Frozen once `Accepted`; never edited in place to match
  later understanding — amended via a dated `## Amendment` section instead (see "Never silently
  rewrite an Accepted decision" below).
- **`spec`** — a living, current-truth document, the opposite of an ADR: edited in place, a
  substantive change is a `Version` bump plus a revision-log entry, not a new record. `spec` is also
  what any *other* type's own schema is defined in (see above).
- **`milestone`** — this project's own backlog: decided-but-unbuilt work, explicitly deferred work,
  unrouted research findings. *Planned* work moving toward done.
- **`bug`** — a real defect already found, retrospective rather than planned, with a required
  `Regression-test` pointer so "nobody verified this is actually fixed" can't happen by accident.
- **`waiver`** — a reviewed, time-boxed exception to a specific rule finding, never a config-level
  ignore list. A waived finding still appears in `check`'s output, only excluded from the blocking
  exit code.

The typical pipeline: an idea becomes an `rfc`; an `adr` derives from it to accept or reject; an
accepted decision that isn't built yet becomes a `milestone` (`Implements` pointing at the `rfc`/`adr`
it realizes); a defect found along the way becomes a `bug`; a `spec` is created or updated once the
resulting schema/behavior is real and needs documenting as current truth. Not every change visits
every stage — a same-turn bugfix still gets a `bug` record even though no RFC preceded it.

`Implements`/`Derives-from`/`Amends`/`Parent` are **pointer fields**; `Blocked-on` is a **narrative
field**. Both are resolved by `pointer.resolution` the same way — a stale or unresolvable reference
is a finding either way, not just a broken link. The real difference is format and what else runs on
top: a pointer field must hold clean, comma-separated references only (`header.pointer-field-clean`
enforces this; it never applies to narrative fields, which tolerate free prose around the reference),
and a narrative field gets one more check narrative fields don't: `narrative-field.stale` flags it
specifically once the record it names reaches a terminal status, a stronger, more specific signal
than `pointer.resolution`'s routine "resolves; target status = X". Use a pointer field for a
relationship that should read as a clean, enforced reference; narrative when it's prose that happens
to name a record, with staleness worth flagging once that record is done.

## Git workflow

Never commit directly to `main`. Create a feature branch and open a PR for every change, however
small — including a one-line docs fix. Check `git branch --show-current` before your first edit if
there's any doubt which branch you're on. Cut the branch from an up-to-date `origin/main`, not from
whatever a prior round left checked out — a branch cut before the last PR merged carries a stale base
and reopens whatever that PR just fixed.

## The review loop

This project's own defects are found by running reviews against real code, not by reading a diff and
trusting it — the discipline in "Verify before trusting" applies to a *reviewer's* claims exactly as
much as to a change's own author. The loop:

1. **Run a review.** This repository has `CodeRabbit` wired in at the GitHub level
   (`.coderabbit.yaml`) — trigger it by commenting `@coderabbitai review` on an open PR, regardless of
   which tool or agent is doing the triggering. If your own environment provides an additional review
   capability (a slash command, a separate reviewing agent), use it too — that's a capability of your
   tooling, not a project requirement, so don't assume the next agent working here has the same one.
2. **Verify every finding against the actual code before acting on it** — a review has produced
   fabricated findings before, and has also, this project's own history shows, missed a real defect a
   narrower earlier check should have caught. Neither "the reviewer said so" nor "I already checked
   this shape once" is verification; read the exact lines the finding names and confirm the failure
   scenario reproduces, or doesn't.
3. **Triage each verified finding into exactly one of three outcomes** — never a fourth "note it and
   move on":
   - **Fix it now** if it's mechanical and needs no design or schema decision. Ship it with a
     planted-violation test, observed failing before the fix and passing after (see "Verify before
     trusting").
   - **File it** (`bug` for an already-real defect, `milestone`/`rfc` for something requiring a design
     call) if fixing it means deciding something, not just correcting something. Don't rush a design
     decision to close a review round faster.
   - **Refute it** if it describes already-decided, deliberate design — `grep docs/adr/` for the
     field, rule, or behavior the finding names; an `Accepted` decision that already made this exact
     tradeoff means the finding is wrong, not the code.
4. **Watch for a repeating shape across findings**, not just each one individually. The same
   collapse-of-distinct-states defect (`BUG-125`) was independently found in three different rules,
   across two separate review passes, before it was fixed once in the shared mechanism instead of
   three times at the call sites. Two independent findings with the same underlying shape is a signal
   to fix the mechanism (or at least name the pattern to the user) rather than patch each instance and
   move on.
5. **Ship each round as one PR**: fresh branch off `origin/main`, a changeset (`.changeset/*.md`)
   per PR describing what an adopter would care about — skip it for the same cases "Before calling
   anything done" already exempts (CI config, internal refactors, docs-only changes) — then `gh pr
   comment <n> --body "@coderabbitai review"` after pushing, `gh pr checks <n>` polled until
   resolved. **Never merge or force-push** —
   that action belongs to the human running the session, every time, with no standing exception.

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
- **Before hand-writing a field the tool already computes, run the subcommand that computes it.**
  `urzua fix --apply` derives `Embodiment` from `Realized-by`'s actual tier — don't hand-assert
  `Verified`/`Drift detected`/etc. yourself and skip the command that exists to get it right.
- **`urzua new`'s assigned display number is provisional, not reserved.** It's a scan of the local
  filesystem, blind to numbers already claimed by other open, unmerged branches or PRs (MILE-89) — a
  real collision this project has hit live. There's no automated check for this yet (MILE-89 is
  unbuilt): before opening a PR, manually check other open PRs for the same type/number — e.g. `gh
  pr list --search "in:title <TYPE>-<N>"` or `git log --all --oneline -- 'docs/<type>/<TYPE>-<N>-*'`
  — a collision found this way is expected and cheap to rename, not a bug in the tool.
- **Before hand-writing any record header or field, check `.urzua/config.yaml`'s
  `header_shape`/`known_fields`/`pointer_fields`/`narrative_fields` for that type.** Declared config
  is the source of truth for header shape, field vocabulary, and relationship-field behavior — never
  freehand an annotation convention (e.g. baking a status or explanation into a pointer field's own
  value) because it looks consistent with nearby records; check what's actually declared, and check
  it again against any rule you've just built or fixed in the same session, since matching an
  anti-pattern you just banned elsewhere is a real, observed failure mode. (`pointer_fields`/
  `narrative_fields`, MILE-90/ADR-44: a type declaring either must declare both explicitly, even as
  `[]` — omitting one is a `config.pointer-declaration-missing` finding, not read as "zero fields, on
  purpose." Every field named in either list must also be in that type's `required_fields`/
  `known_fields`.)

## Verify before trusting

This project's own history is full of defects found by actually running the tool against real
input, not by reading the code and assuming it does what its comment says. Before treating a claim
about behavior as true:

- Run the actual command against a real or disposable fixture. `check <path>` looking identical to
  `check docs/` was found this way — nothing about reading the code would have surfaced it.
- A new rule or fix needs a **planted-violation test, observed failing before the fix, then passing
  after** — not just a test that always passed. If you can't show it failing on the old code, it
  isn't verified yet.
- A test written **after** the code it covers has no failing state to observe, so **break the code
  and watch the test go red**, then restore. Same standard, opposite order. Three assertions in the
  `0.4.0` work passed against the bug they were written for — a population pinned to 100%, an
  `accepted || reported` pair that are exact complements, and a determinism check looping over one
  `HashSet` whose order is fixed within a process. Each derived its expected value from the same
  source as the thing under test, and reading them did not reveal it.
- Don't assume a design decision matches what's actually implemented. Grep the code, don't just
  read the ADR.
- When a change invalidates a claim in prose, grep the whole file (or corpus) for that claim's key
  phrase before calling the change done — not just the section being actively edited.

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
underlying issue is. Same rule as the stale-shim case above, applied to a live checker finding
instead of something you noticed by reading code — in both cases, file the defect, don't quietly
absorb it.

## Before calling anything done

- `make ci` passes locally (fmt, clippy, build, test, `urzua check docs/`).
- A changeset exists in `.changeset/` for anything a person installing `urzua` would care about
  (ADR-29) — skip only for CI config, internal refactors, or docs-only changes.
