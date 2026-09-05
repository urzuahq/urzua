# 0011 — Declared boundaries: a policy is enforced at the width it declares, by every tool that touches it

> Status: Accepted
> Date: 2026-08-11
> Author: (session author)
> Supersedes / Superseded-by: —

> **Partial acceptance, 2026-09-04.** §§1–5 (boundaries as declared facts, severity,
> violation-vs-evidence separation, mechanical width comparison, planted-violation tests for
> absence checks) are decided — see [ADR-0009](../adr/0009-declared-boundaries-and-the-width-of-enforcement.md).
> §6 (is scope a path glob or a subject?) remains explicitly open, per this RFC's own recorded
> uncertainty — ADR-0009 declines to decide it rather than assume an answer the evidence doesn't
> support yet.

## Summary

A record identifier is only meaningful to a reader who can resolve it. Code that travels beyond the
corpus — a public mirror, a published package, a vendored copy, a generated site — carries
references that resolve to nothing for its actual audience, and carries governance markers into
places they were never meant to reach. Propose that Urzua treat this as a first-class declared
fact: a **boundary** names a set of paths that publish beyond the corpus, and every tool that
touches those paths reads that one declaration. Propose further, and more generally, that a
declared policy carries a **width** — what the declaration claims — which must be mechanically
compared against what its detector actually matches, because the gap between the two is invisible
by construction and produces a check that passes forever against the wrong question.

Propose finally that a record's own **scope** — what it governs — is a declaration of exactly this
kind: global identity, free location, scope as a declared set of paths rather than encoded in a
number or inferred from a directory. Scope in the identifier makes back-pointers ambiguous and
disables the realization axis; scope in the path changes silently when packages are renamed.

## Motivation

Consider a case that is unusually clean evidence for this failure mode, because everything needed
to catch it was already in place, and it still failed.

A publicly mirrored package accumulates decision-record citations over time — comments like
`(ADR-0026)`, `see ADR-0028`, `per ADR-0028`, scattered across several records. The rule against
this is written in the agent-instruction file **and** declared in the linter's committed config,
with a reason string: *internal governance/decision references don't belong in published source at
all.* The check runs on every audit and passes every time.

It passes because its detector matches only the two structured back-pointer markers
(`Implements:` / `Verifies:`), while every one of the citations is prose. **None of the offending
lines are in the catchable form.**

Three failures, each necessary:

1. **Width.** The config's prose claims *references*; the code matches *markers*. Both sentences
   are true; only one executes.
2. **Severity.** Violations print a count. No flag or config key makes them block — the audit's
   `--fail-on-drift`, which CI and the pre-commit hook both pass, gates drift only.
3. **Duplication.** The new guard written to close the hole hardcodes the boundary path instead of
   reading the config that declares it. Two enforcers, one configured and one literal.

**The mechanism worth generalizing** is why the check looked healthy. The catchable form had zero
instances because of an *earlier fix*: the record-side `Realized by:` pointer (RFC-0005 §2) was
introduced partly so that this very package would never need `Implements:` comments. The mitigation
for the boundary problem in the back-pointer direction guaranteed the detector for the citation
direction would find nothing — and the blindness presented as a clean bill of health.

This is a harder-to-detect sibling of the silent no-op SPEC-0001 already treats as a first-class
failure: there, a check examines an empty set and reports zero. Here it examines a real, populated
set and reports zero, **correctly**, against a narrower question than the one it was believed to be
asking. Instrumenting "did this check run" — SPEC-0001's own answer to the first case — would not
have caught it. It ran.

This is also a scope assumption in the same family RFC-0010 names at header granularity (a field
lookup assuming the header is the whole document) — a reference assuming one universe of readers is
the same mistake, one layer out. A cross-record coherence check can make the identical error by
assuming decision and code share a branch. Urzua should assume none of these by default.

## Proposal

### 1. A boundary is a declared fact, not a rule

Config declares boundaries: a set of path globs, each with a kind (`published`, `vendored`,
`generated`) and a required human-readable reason. That is the whole declaration. It says where
things go, not what is forbidden.

Prohibitions derive from it, which is the test that the concept is right rather than invented — one
declaration, several consequences:

- A record identifier appearing inside a `published` boundary is a **violation**: it publishes a
  pointer no external reader can resolve.
- A realization marker inside a `published` boundary is **not evidence** (see §3), independently of
  whether it is also a violation.
- A `generated` boundary is not a place a human-authored back-pointer may live at all, since it is
  overwritten.

If prohibitions were configured directly, each would need its own key and the set would drift.

### 2. Severity is part of the declaration

Each boundary declares whether violations block. §30's boundary had no way to say "this reaches a
public artifact, so failing is correct" — the only severity available was informational, and the
tool's one blocking flag gated a different check entirely. A boundary that cannot be made blocking
is a preference, not a policy.

### 3. "Violation" and "evidence" are separate questions about the same text

The single most consequential implementation constraint, and it is not obvious.

`// Implements: ADR-0042` inside a published path is two things at once: a marker the embodiment
computation would count as proof the decision was built, and a governance reference that should not
be in published source. These require **different matchers**, and merging them is a live bug:
widening the shared detector so it catches prose citations would silently begin counting prose
mentions as realization evidence. A decision would be marked `Implemented` because someone wrote
"see ADR-0042" in a comment.

So: the boundary check uses a wide matcher (any record identifier); the evidence collector keeps its
narrow, structured one. Same text, same boundary, two questions, two matchers, deliberately.

### 4. Declared width must be mechanically comparable to matched width

The general form of failure (1), and the part of this RFC that outlives the boundary feature.

A config key whose meaning lives in adjacent prose has a stated width and an implemented width, and
nothing compares them. Proposed mitigations, in increasing order of ambition:

- **The reason string is specification, not decoration.** Treat it as part of the contract under
  review, not a comment.
- **Every policy declares its detector's width in machine-readable form** — the pattern class it
  matches (`structured-marker` vs `any-reference`) — so "this policy claims references, its
  detector matches markers" is a mechanical mismatch rather than a reading-comprehension exercise.
- **A policy with zero lifetime hits is reported as unverified**, not as clean (see §5).

**The comparison runs in both directions, and the second direction is the one nobody looks for.**
This section, and the failures that motivated it, describe *declared wider than enforced* — a
boundary promising more than it checks. The inverse is the same defect: a rule that is **enforced
and declared nowhere**.

Consider that case at scale: a required field added across dozens of records and enforced by a
blocking check with a fixed vocabulary that appears in no document a record author would read;
separately, several checks that could fail a build are absent from the contract document listing the
blocking checks. An author's first contact with either rule is the error message.

The asymmetry matters for tooling. Declared-but-unenforced is found by anyone who tests the claim.
**Enforced-but-undeclared is found only by someone who hits the rule and then goes looking for it** —
and the people who hit it are the least equipped to know it should have been written down. Nothing
surfaces it from the inside, which is why it needs a mechanical check rather than review.

So the requirement is bidirectional, and each direction catches a different mistake:

- **Every rule the checker can emit appears in the contract document.** Catches a rule nobody
  documented.
- **Every rule the document names is one the checker can emit.** Catches documentation for a rule
  that no longer exists — worse, because it reads as coverage.

The same pair applies to any closed vocabulary a checker enforces (statuses, scopes, record types),
not only to rule identifiers. A checker that can enumerate its rules to run them can be asked to
prove they are documented; the assertion is a few lines on top of the registry `specs/0002-urzua-check.md`
already requires. Each direction must be observed failing on a planted violation before it is
trusted — §5's requirement, applied to the contract check itself.

### 5. An absence assertion is unverified until its failure has been observed

An absence check of the form "there are no X" carries no information until someone has watched it
fail. Two structural requirements:

- **Every absence check ships with a planted-violation test.** Not a nicety — the only evidence the
  check can fail at all.
- **A shell-based check must fail closed.** `git grep` exits 1 on no-match — a failure code for the
  passing case. `catch { return [] }` yields a check that passes forever regardless of the tree.
  Only the specific no-match status may be swallowed; anything else propagates.

Urzua should additionally surface a check that has **never** produced a finding across the corpus's
recorded history as *unverified* rather than *passing*, which is the only mechanism that would have
surfaced §30 before the citations reached the mirror.

### 6. A record's scope is a declared boundary, not a path and not a number

The same declaration shape answers a question the corpus has not yet had to face: in a monorepo,
what does a record govern?

Three schemes are available, and two of them encode the answer in the identifier.

| Scheme | Identity | Scope lives in |
|---|---|---|
| Per-package sequences — `SPEC-0002-web`, `SPEC-0002-api` | Package-local | The number |
| Co-location alone — `apps/web/specs/0002-*.md` | Path-derived | The directory |
| **Global identity, declared scope** (proposed) | Global and stable | A field |

**Scope must not live in the number.** A per-package sequence reproduces, once per package, the
collision that `highest + 1` already produces once per corpus — and adds a failure the global
scheme does not have. `Implements: SPEC-0002` stops resolving when three packages each have one, so
every back-pointer needs qualifying, and back-pointers are the entire mechanism by which Embodiment
is computed (RFC-0005). A scheme that makes the identifier ambiguous disables the realization axis
to save a directory listing.

This is not hypothetical: RFC-0013 treats a vacated identifier as a first-class record precisely
because a single global sequence produces renumberings and permanently vacated numbers in practice,
including a collision caught only during review. Multiplying sequences multiplies that cost, and the
vacancies become per-package too.

**Scope must not live only in the path either.** A directory is a location, and locations are
mechanically indistinguishable from claims: a record under `apps/web/specs/` might govern that
package, or might merely have been written by someone working there. More practically, a package
that is renamed, merged or split silently changes what every record inside it appears to govern,
with no diff that says so — a structural change with a blast radius outside the file it is made in,
which is the failure class SPEC-0001's acceptance suite already records.

**So: identity is global, location is free, scope is declared.** A record carries `scope` as a set
of path globs, exactly the shape §1 gives a boundary. `SPEC-0007` with `scope = ["apps/web/**"]`
may live at the repo root, beside the code, or anywhere else; moving it changes nothing about what
it governs, and changing what it governs is a visible edit to a declared field.

This makes §4's width rule apply directly, which is the point of putting it here rather than in an
RFC of its own. A declared scope is a stated width; the set of places that actually cite the record
is the matched width; and the two are mechanically comparable:

- A record cited from outside its declared scope is either a miscited record or a scope that was
  never widened to match practice. Both are worth a finding, and the tool cannot tell them apart —
  so it reports the mismatch and names both readings, rather than guessing.
- A record whose declared scope matches **no path in the tree** is unverified, not clean: §5's rule,
  applied to scope. A glob pointing at a package that was renamed away is the common case.
- Two records with overlapping scope and contradictory claims are the cross-record question this
  makes answerable at all. Without a declared scope there is no way to know two specs are talking
  about the same thing.

**Tooling this makes possible**, none of which needs a new concept:

- `urzua explain <path>` — the roadmap's highest-value read command — becomes a scope lookup rather
  than a heuristic: which records declare a scope matching this file.
- `check` gains scope-mismatch as a rule alongside the boundary rules, sharing the resolver.
- `new` can infer a default scope from where the author is working, as a **suggestion written into
  the field**, never as an implicit fact derived from the path at read time. The distinction is the
  whole of this section: a value in a field can be reviewed and can be wrong out loud.
- Scope resolution is one function — globs against tracked paths — reused by all three, which is the
  test that this is one concept rather than three features.

#### Unresolved: a strong counter-argument says the value should be a *subject*, not a glob

**This is a substantive challenge to the proposal above, recorded rather than resolved.** The
scheme argues identity-global / location-free / scope-declared, and then makes the declared value a
set of **path globs** — which keeps the answer path-shaped, one indirection later.

A scope field applied across a real multi-type record set might instead choose subjects (`engine`,
`ui`, `platform`, `devops`, `governance`) over paths, for reasons that bear directly on the text
above:

- **A path breaks on the rename this section already names as the failure case.** §6 rejects
  location-derived scope because a renamed package silently changes what records appear to govern.
  A glob has the same exposure: it points at a location, so the same rename leaves it pointing at
  nothing. The RFC's own §5-derived rule — *a scope matching no path is unverified, not clean* —
  is a mitigation for a problem the subject form does not have. A corpus that renames its record
  homes even once in a working session already exercises this gap.
- **The question a glob answers is already answered better.** "Which code realizes this record" is
  what `Implements:` back-pointers give, per file, maintained by the people editing the code, and
  it is the mechanism RFC-0005 already relies on. A glob is a second, hand-maintained answer to a
  question the claim graph answers mechanically.
- **A subject survives refactors that a path cannot.** "What is this record about" does not change
  when a directory moves.

There is also a cheap test for whether a candidate value is genuinely a subject: **can a second
instance of it exist?** A value naming one of several sibling packages reads as a subject on a
casual pass and is falsified only by counting the packages. Values naming things there is one of
(`engine`, `governance`) pass; values that are directory names in disguise fail.

What the glob form does buy, and the subject form does not, is `explain <path>` as a direct lookup
and a mechanical stated-vs-matched comparison — the two capabilities this section is built around.
The subject form gets `explain` from back-pointers instead, and gives up the width comparison for
scope specifically.

**Both readings are live.** A resolution needs the second corpus (Q12), and the honest position is
that the glob proposal has not been tested against a corpus that renames, while the subject form
has not been tested against `explain`.

### 7. A second, independent field family generalizes the mechanism — not the open question above

The same width check generalizes beyond scope to any closed, declared field family. Relations
(`Supersedes`, `Depends on`/`Feeds into`, `Requirements`, and others) are a *different* field family
from scope: a relation field may only target a record, the set of relation fields is closed and
named, and reciprocity should be checked bidirectionally — with the added property that a check
running against only one record type still catches a violation whose only trace is a claim in the
*other* type's header, via a lookup populated by both linters before either runs its own checks.

This does not touch §6's open question — relations and scope are different fields, chosen
independently, and nothing about relations bears on glob-vs-subject. What it does illustrate is the
claim §6's whole proposal rests on: a closed, declared field set with a mechanical
stated-vs-matched-width check generalizes beyond a single field family, not just scope specifically.

### Fail-open and fail-closed are decided by what kind of rule it is

Width is not only about how much a boundary covers. It is also about what
happens when the boundary's *input* goes missing, and the right answer inverts
depending on the rule.

A **detector** — dangling references, drift, orphaned records — must fail open.
Empty discovery means "we could not enumerate the corpus", not "the corpus is
empty", and resolving against an empty set accuses every reference at once.

A **prohibition** — this number is retired, this value is forbidden — must not.
An empty set of prohibited values permits everything, and reports the same
silence as a corpus that complies. A concrete instance of the failure: a rule
reads its prohibited set out of a documentation table by column position, and
returns an empty set when the table moves. Reordering the table changes what
is prohibited; renaming a heading disables the rule outright. Both states are
indistinguishable from compliance.

Two corrections generalise. Locate structured data in prose by **heading**, not
position, so a document's presentation and its meaning are decoupled. And return
the *reason* alongside the result, so the checker can report "I could not read
the rule" rather than "the rule is satisfied".

A confirming instance for the detector side: a pre-write hook that checks a proposed `Embodiment`
edit against realization evidence computed by a separate audit step should fail open when that
audit's output is missing or stale — a detector, by this section's own classification, correctly
built to fail open rather than block every edit on an absent artifact.

### A declared exception needs a check that it still applies

Declared asymmetry is the mechanism this RFC relies on: a difference between two
enforcement surfaces carries a written reason. But the assertion that guards it
is naturally one-directional — everything asymmetric is declared — and that
cannot see a declaration whose asymmetry has since disappeared.

Two such declarations were found carrying confident, obsolete reasons for rules
that both surfaces had come to share. That is worse than an undeclared
difference, because it reads as considered. The converse assertion is one line:
nothing declared one-sided may be emitted by both sides.

The same hole appears in documentation gates. Asserting that every rule appears
in the contract document proves it is *mentioned*; it says nothing about where.
A catalogue that groups rules by applicability had eight filed under the wrong
group, three of them added while the gate was green. **A gate must compare the
same structure the document asserts** — set membership is the weakest reading of
a structured document and the easiest one to write.

## Open questions

- **Is a record's declared scope a path glob or a subject?** §6 proposes globs; the first field
  application chose subjects, on the grounds that a glob is location-shaped and inherits the rename
  failure §6 rejects location-derived scope for. The trade is `explain <path>` and the stated-vs-
  matched width comparison (globs) against rename-survival and no second hand-maintained answer to
  a question back-pointers already answer (subjects). Unresolved; needs the second corpus.
- **What does a legitimate reference become inside a boundary?** Stripped, inlined as prose, or
  rewritten to a public URL. "No references at all" is the simplest answer and works for a
  code-only mirror, but will not generalize to a published SDK whose docs genuinely want to explain a
  decision. A public-URL rewrite is the obvious candidate and implies records have public identities,
  which is a much larger commitment.
- **Can boundaries be derived rather than declared?** Mirror configuration, `package.json` `files`,
  and publish workflows already encode which paths escape. Deriving would remove the drift between
  declared boundary and actual publication — and add a dependency on parsing every publishing tool.
  Declared-with-optional-verification is the likely middle.
- **Is "never fired" measurable in practice?** §5's proposal needs per-check hit history, which is
  state Urzua does not currently keep anywhere. Cheap for a hosted service, awkward for a local CLI.
- **Is scope one field or two — what a record governs versus where it may be cited from?** §6 treats
  them as one. They come apart for a decision that is *made* in one package and *binding* on
  several: an infrastructure choice governs the whole tree but is cited from one place, and a shared
  convention is the reverse. One field forces the author to pick a reading; two fields ask every
  author to think about a distinction most records do not have.
- **Does an unscoped record mean "governs everything" or "scope not yet stated"?** These need
  opposite treatment — the first is a legitimate corpus-wide decision, the second is an incomplete
  record — and they are indistinguishable in an absent field. This is the general blank-versus-pending
  distinction arriving on the scope axis. An explicit corpus-wide value is the likely answer, which
  makes an absent field always a defect.
- **Do nested scopes inherit?** If `SPEC-0007` scopes `apps/**` and `SPEC-0012` scopes
  `apps/web/**`, is the second an exception to the first, a refinement of it, or a conflict? Answer
  affects whether overlap detection is a warning or an error, and the corpus has no instance of this
  yet to reason from.
- **Does the wide matcher produce false positives on the corpus's own tooling?** A linter's test
  fixtures can deliberately contain record-shaped strings — a self-referential-fixture trap the
  test harness must guard against directly (fixtures live outside the discovered record set, and
  that exclusion is asserted, not assumed). Whether boundaries exclude them by construction or by
  luck needs specifying, and the interaction between boundary globs and the ignore-marker convention
  needs the same treatment.

## Non-goals

- **Deciding any org's mirror policy.** Urzua enforces a declared boundary; it does not have an
  opinion on what should be published.
- **Rewriting or stripping references automatically.** That is RFC-0008 territory and needs the
  answer to open question 1 first.
- **General secret/PII scanning of published paths.** Adjacent, well served by existing tools, and
  not decision-record shaped.
- **Cross-repo reference resolution.** Roadmap Phase 4. This RFC only detects that a reference has
  left its resolvable scope; making it resolvable elsewhere is a different feature.

## References

- RFC-0005 §2 — the record-side pointer whose introduction can cause the blindness described in the
  Motivation.
- RFC-0010 — the sibling scope assumption, at header granularity.
- `docs/specs/0001-v0-cli.md` — no-silent-no-op, which §5 extends from "did it run" to "has it ever
  failed."
- RFC-0013 — a vacated identifier treated as a first-class record, the concrete cost §6 declines to
  multiply per package.
- ADR-0003 — stable identity separated from display number, which is what makes §6's "location is
  free" affordable rather than aspirational.
- RFC-0005 — the back-pointer mechanism that a package-scoped identifier would make ambiguous.
