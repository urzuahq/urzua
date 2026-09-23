---
Version: '0.2'
Date: 2026-09-23
Status: Accepted
Author: beauwilliams
Stable-Id: 01M367MCQSWWBSJBJFYQZ12ZXD
Subject: 'Rust coding conventions actually already in force in this codebase, backlinked to where each was decided or landed.'
Parent: —
Implements: ADR-1
---
# SPEC-22 — Rust coding standards

## Purpose

`ADR-1` decided Rust as the implementation language, for distribution and startup-cost reasons — it
says nothing about how Rust is written once chosen. No such document existed before this one: a
repo-wide search (commit history, `AGENTS.md`, `CONTRIBUTING.md`, every `ADR`/`RFC`/`SPEC`) turned up
zero mentions of coding style as its own subject. The conventions below are not proposed here; every
one is already load-bearing in `rust/`, surveyed directly from the code and from the commits that
introduced or corrected it. This spec's job is to name them in one place and backlink each to its
origin, not to invent new rules.

## Conventions

### Domain values compare themselves; nothing else may

A value the adopter declares (a field name, a record id) is wrapped in a newtype whose `Eq`/`Hash`/
`Ord` *is* the comparison rule, so the rule is stated once instead of re-implemented at every call
site. `FieldName` compares exactly (`ADR-57`); `RecordId` normalizes numerically (`BUG-2`'s
precedent). Before this, the same comparison lived at six call sites across three modules, and
between the first pass and the last, two rules disagreed about whether one field was declared
(`RFC-39`/`ADR-59`, `MILE-107`, landed in `fa4e522`).

**Why:** `MILE-107`'s own record states the evidence plainly — `Population` (below) has never been
wrong across the whole 0.4.0 cycle; a hand-applied comparison was wrong at least three times
(`BUG-59`, the field-set-consistency/required-fields disagreement, `BUG-2`). A type makes the
comparison rule impossible to restate incorrectly; a convention does not.

### A bare collection does not cross a function boundary

Ten rule signatures that took a raw `&HashMap<String, Vec<String>>` (or similar) converged on taking
`&Config` directly, or a named projection type, matching the pattern fifteen other rules already used
(`RFC-39`/`ADR-59`, `MILE-107`). `field.untrimmed-value` was wired to the wrong same-shaped map
(`known_fields_by_type` instead of `declared_fields_by_type`) and the compiler had no way to catch
it — both are `&HashMap<String, HashSet<String>>`, identical in type, opposite in meaning.

**Why:** identical types with different meanings are a silent-swap hazard the type system can close.
Preferred fix, in order: pass `&Config` and project inside the function; failing that, a named struct
or newtype wrapping the collection, never a second `HashMap<String, Vec<String>>` parameter next to
the first.

### One declared population, computed once, never hand-maintained beside a filter

Every rule reports what it was eligible to examine and what it actually judged via `report::census`/
`census_records`, which builds `Population` from the real candidate list — never a hand-incremented
counter kept beside a separate filter. `Population`'s fields are private with one constructor path
(`Population::detailed`/`of`), so `examined <= eligible` is structural, not merely checked after the
fact: a public-field struct would let any caller write `Population { eligible: 2, examined: 5, .. }`
directly.

**Why:** `records_examined` — a bare `usize` maintained beside an independent filter — was wrong four
times in one release (`BUG-78`, `BUG-81`, `BUG-83`, `BUG-90`). `Population`, built structurally from
the same candidate list the rule body iterates, has not been wrong once. Landed as "convert the last
seven hand-built populations to census" (`2e39054`) and carried into `fix`/`migrate` (`64e61d5`).

### Duplicated logic is extracted on its second occurrence, not its third

`field_slots`, `declared_slots_for_roles`, and `pointer_and_narrative_slots` are the one definition
each of "which `(record, field)` pairs are this rule's candidates," shared across 8+ rule functions
that used to each hand-roll the same `filter_map`/`flat_map` chain. `census`/`census_records`,
`Config::sorted_type_names`, `resolve_inside_repo`, `RecordTypeConfig::has_no_header`, and
`claim_status_agreement_setting` are the same fix, each closing a different pair of hand-rolled
copies once found.

**Why the rule changed, not just the examples**: this section originally said "consolidate once real
duplication has caused a real bug, not in anticipation of one" — extract on the *third* copy, after
the first drift is already shipped. That was `BUG-105`/`BUG-106`'s own lesson, applied literally. It
was the wrong lesson. In the same review cycle that produced every example above, two *more*
instances of the identical shape were found and left unfixed rather than rushed (`BUG-133`:
`is_record_reference`/`parse_record_filename` re-implementing the same identifier grammar in two
places, already caught drifting once by `BUG-114`; `BUG-134`: two functions rebuilding a per-type
constant `declared_fields_by_type` already caches). A rule that waits for the second drift to happen
before naming the first copy as the problem will keep producing `BUG-105`/`106`/`114`-shaped defects
indefinitely — the fix has to fire when the *duplicate* is written, not when it disagrees.

**The rule now**: when a second call site needs logic a first one already has, extract a shared
function in the same change, before either site is touched again — don't wait for a third copy or a
live bug to justify it. This doesn't relax `AGENTS.md`'s "don't build speculative capability": that
principle is about not building a *capability* nothing has asked for yet; this is about not leaving a
*fact already stated once* restated a second time, which is never speculative — the second call site
is real, present evidence the fact needs a name.

### One index, one normalization pass, per run

`build_normalized_index` is computed once per `check` invocation and threaded to every rule that
resolves a reference against it, replacing five independent per-rule traversals of the same record
set. `identity.collision` was the one rule still rebuilding it privately after the other five moved
(`BUG-116`) — found by review, not by design, which is itself the argument for the shared function
over "remember to reuse it": a reviewer has to notice the omission by hand every time a new call site
is added.

**Why:** landed as "Consolidate duplicated normalized-record indexes into one per run" (`41bb4ab`,
`353327c`); `BUG-116` is the shape of defect that recurs when the discipline is convention rather than
a function signature that makes the shared index the only one in scope.

### A declared-field lookup miss has one contract, not one guess per caller

`Header::read_declared` returns `FieldRead::{Present, Missing, Unreadable}` — a caller cannot
`.unwrap_or("some sentinel")` a fabricated value the way an `Option<&str>` invited. Before this
(`RFC-40`/`ADR-58`, `5e92a36`), a lookup miss was answered differently by different call sites, and at
least one fabricated a sentinel string (`"(no Status field)"`) that then matched no configured value
and produced a false positive on every record it touched (`BUG-100`).

**Why:** an `Option<T>` at a boundary with more than one kind of "not there" (absent vs. unparseable)
invites each caller to invent its own answer. An enum whose variants are the actual distinct meanings
closes that gap once, at the type, not per call site.

### No `unsafe`, anywhere, in any crate

Every crate carries `#![forbid(unsafe_code)]` at its root (`urzua-core`, `urzua-io`, `urzua-id`,
`urzua-agdr`) — not `#![deny]`, which a local `#[allow]` could override, but `forbid`, which cannot be
overridden from inside the crate.

**Why:** this is a governance tool that reads and reasons about a host repository's own records; the
engine has no performance requirement that would justify the risk, and `ADR-1` never named raw speed
as a reason to choose Rust in the first place — only startup cost and static distribution.

### The impure half is a separate crate; the core takes no filesystem, no `Path`

`urzua-core` never touches a `Path` — filesystem and `git` access live in `urzua-io`, and the CLI
(`urzua-cli`) is the only crate that sees both. `read_claim_files`, `compute_drifted_records`, and
similar I/O live in `urzua-cli/src/discovery.rs` precisely because `urzua-core`'s rules are pure
functions over already-loaded data (`ADR-5`, `ADR-6` for why `git` is shelled out to rather than
linked as a library).

**Why:** a pure core is exhaustively unit-testable without a filesystem fixture for every case, and
it is what makes property tests like `purity.rs`'s own dependency-graph check meaningful — nothing
downstream of `urzua-core` needs to reason about I/O ordering, partial writes, or process spawning.

### Extract the assertion as a pure function so a planted violation can prove it fails

`purity.rs`'s `check_dependencies_are_allowed`, `relation_field_literals.rs`'s `find_violations`, and
every rule's own body follow the same shape: the actual comparison is a free function over
already-collected data, called once against the real input and once against a synthetic violation.
`CONTRIBUTING.md` already states the resulting rule for rules themselves — *"every rule ships with a
planted-violation test... a rule without one is unverified, not trusted"* — this generalizes it to any
mechanical check in the codebase, not only `check`'s own rule set.

**Why:** a check that has only ever been observed passing has not been shown capable of failing. A
free function taking plain data (not a live `Config`/filesystem) is what makes the planted-violation
case cheap enough to always write.

### A comment states the invariant, never the diff

Comments explain a hidden constraint, a subtle invariant, or a workaround for a specific defect (bug
ID, ADR, or RFC cited by number) — never what the code already says by being read, and never the
history of how it got there ("previously," "now," "this used to"). A temporal-language comment was a
filed, fixed defect in its own right (`BUG-107`): *"They previously carried a recogniser each..."*
was corrected to state the invariant directly instead of narrating the change that produced it.

**Why:** a comment describing the diff instead of the invariant goes stale the moment a second diff
lands on the same code, and nothing catches it going stale — the invariant is the only part of a
comment with a truth value that outlives the commit that wrote it.

### Error types: `thiserror` at a library boundary, a plain message at the CLI boundary

`DiscoveryError`, `ConfigError`, and similar are `#[derive(thiserror::Error)]` enums naming exactly
what failed and where, used by every library crate (`urzua-core`, `urzua-io`, `urzua-id`,
`urzua-agdr`). `urzua-cli`'s own command functions return `Result<T, String>` at their own internal
boundaries, where the message *is* the full user-facing text and nothing downstream inspects the
error's shape — it terminates in `CouldNotRun::from` and is never matched on.

**Why:** a structured error type earns its cost where a caller might need to distinguish failure
kinds programmatically (a library boundary). At a CLI command's own internal plumbing, where the only
consumer is `emit(&CouldNotRun::from(e))`, a `String` is not a shortcut — a `thiserror` enum there
would be unused structure.

## What this spec does not do

- **It does not introduce a new lint policy.** `make ci`'s `cargo clippy --all-targets -- -D warnings`
  uses clippy's default lint groups only; this spec doesn't propose enabling `clippy::pedantic` or
  `clippy::nursery`, and none of the conventions above are enforced by lint today — they are conventions
  a reviewer (human or agent) checks by reading, the same as everything else in `AGENTS.md`.
- **It does not retroactively audit the whole codebase for violations.** Each convention above is
  cited from where it was decided or fixed; it is not a claim that every line in `rust/` already
  conforms.
- **It does not cover process** (branching, changesets, PR review) — that is `AGENTS.md`'s and
  `CONTRIBUTING.md`'s territory; this spec is code shape only.

## References

- `ADR-1` — Rust as the implementation language; this spec's parent decision.
- `RFC-39`/`ADR-59`, `MILE-107` — domain-value types and the `&Config`-projection convergence.
- `RFC-40`/`ADR-58` — `FieldRead`, the declared-field-miss contract.
- `ADR-57` — exact field-name comparison.
- `BUG-78`, `BUG-81`, `BUG-83`, `BUG-90` — the hand-maintained-population defects `census` replaced.
- `BUG-105`, `BUG-106`, `BUG-116`, `BUG-114`, `BUG-133`, `BUG-134` — the duplicated-selection-logic
  and duplicated-index defects the shared-helper convention exists to prevent recurring; the last
  three are the evidence the convention's own timing rule was too late.
- `RFC-7` §7 — first named this same shape ("duplicated logic across near-identical tooling
  functions is itself an unenforced drift risk"), inside a proposal about a different, still-`Draft`
  subject (a pre-write enforcement harness for decision-record fields). Moved here rather than left
  there, since this is a coding convention, not part of that harness decision.
- `BUG-100` — the fabricated-sentinel defect `FieldRead` closes.
- `BUG-107` — the temporal-language comment defect.
- `ADR-5`, `ADR-6` — the pure-core / impure-`urzua-io` split, and shelling out to `git`.
- `CONTRIBUTING.md` — "every rule ships with a planted-violation test," generalized here.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed. **Why:** asked directly whether this codebase's Rust conventions were ever written down; a full search of commit history, `AGENTS.md`, and every `ADR`/`RFC`/`SPEC` found nothing. Every convention here was surveyed from code already in `main` and backlinked to the commit or record that introduced it, not proposed fresh. | **substantive** |
> | 2026-09-23 | `Status: Draft` → `Accepted`. Tightened "Duplicated candidate-selection logic gets one shared function, not a copy per rule" (renamed to state the timing rule directly) from "extract after a real bug, not speculatively" to "extract on the second occurrence, not the third" — the same review cycle that produced this spec's own examples also found two more instances of the identical shape and left them unfixed rather than rushed (`BUG-133`, `BUG-134`), which is exactly the reactive rule failing to prevent its own next instance. Absorbed `RFC-7` §7 (the same principle, first named inside an unrelated, still-`Draft` proposal) rather than leaving the same fact stated in two places. **Why:** the user asked directly whether this recurring pattern needed an RFC or ADR of its own; it doesn't — it's a coding convention with no alternatives being weighed, and this spec already existed as its right home. | **substantive** |
