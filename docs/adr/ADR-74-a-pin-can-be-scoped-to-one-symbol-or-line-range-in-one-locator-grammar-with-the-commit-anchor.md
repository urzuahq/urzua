---
Stable-Id: 01M6MA7HNPBGVZ1MFY3NJ6A3NG
Status: Proposed
Date: 2026-11-04
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Derives-from: RFC-5, ADR-67
---
# 74 — A pin can be scoped to one symbol or line range, in one locator grammar with the commit anchor

## Context

`ADR-67` pins a `Realized-by` locator to the hash of the whole cited file: `code:path@sha256:<64 hex>`.
That is exact, but the files worth pinning are the most edited ones, and a decision is almost always about
one function, struct or block in them. `MILE-185` records the evidence and asks for this ADR first, because
a locator syntax written into records cannot be changed without rewriting them.

Today's evidence, from the drift review of 2026-11-04 and the history since:

- The 14 records pinned in #180 (`ADR-18`, `22`, `24`, `26`, `30`, `32`, `35`, `36`, `38`, `39`, `43`, `44`,
  `57`, `SPEC-1`) were all reported stale within about nine hours of being pinned.
- **11 of the 14 needed nothing but a re-pin.** The edits to the cited file did not touch what the decision
  describes: fixtures converted to YAML frontmatter, a struct default, a new option on a sibling rule, one
  added line in `audit.rs`. The other three (`ADR-32`, `ADR-57`, `ADR-38`) had a real divergence in the cited
  code, which a pin scoped to the cited item would still have caught, because the items that changed were the
  ones those decisions are about.
- The churn sits in a few volatile files. Counted on `main` since #180: `.urzua/config.yaml` 37 commits,
  `rust/crates/urzua-core/src/config.rs` 35, `rust/crates/urzua-cli/tests/check_integration.rs` 14,
  `rules/embodiment.rs` 13, `main.rs` 8. `config.rs` is about 3,460 lines; a decision that cites it is
  about one rule's options or one struct, a few percent of the file. A file-level pin on a file that busy
  goes stale on the next unrelated commit, however often it is re-baselined (`urzua fix --rebaseline`,
  `MILE-181`, makes the re-pin cheap to do and does nothing about it being needless).
- Each needless stale pin costs a reviewer a re-verification and a revision row about a change that does
  not bear on the decision. That trains the review into a rubber stamp, the opposite of what a pin is for.
  With `drift_grace_days: 14` the 11 stale pins from the review go red on about 2026-11-18, and the same
  will recur on the next set.

Three constraints shape the answer:

- `ADR-64`: structured in, structured out. A scope is a declared value the engine can resolve and report,
  not prose it guesses at.
- `SPEC-1`'s config-driven claim and no-silent-no-op rule. The engine has no list of languages, and a pin
  that stops pointing at anything is reported, never passed.
- `MILE-84` (Planned) proposes line- and commit-anchored locators (`path@<short-sha>:<line>`). Two features
  that each invent a suffix after the path would collide on `@`, so the grammar is decided once, here.

## Options considered

| Option | Pros | Cons |
|---|---|---|
| A. Keep file-level pins; make re-baselining cheaper | Nothing new to build or learn | The 11 needless re-pins recur on every volatile file; review fatigue stays |
| B. A scope between path and pin: `path#<symbol>` or `path#L<a>-L<b>`, symbols found by resolvers declared in config (chosen) | Pin goes stale only when the cited item changes; no language list in the engine; one grammar that can also carry `MILE-84`'s commit anchor; file-level locators unchanged | Each resolver is config an adopter must write and keep right; a regex resolver is approximate; a line range moves when lines are added above it |
| C. Markers inside the cited file (`// urzua:begin ADR-57` ... `// urzua:end`) | Exact span, survives edits above it | Edits the cited file for the record's sake; impossible in generated, vendored or comment-less files (`MILE-84` names these as the reason for locators); couples code to record ids |
| D. Parse each language (tree-sitter or similar) and hash the syntax node | Precise; whitespace and comment changes do not matter | A hardcoded list of languages and grammars in the engine, against the founding claim; a heavy dependency and a per-language upgrade burden |
| E. Pins as a separate header field (`ADR-67` option C) | Room for per-locator metadata | A schema change to `Realized-by`, which `ADR-18` kept flat; `ADR-67` already declined it |

## Decision

In the context of file-level evidence pins that go stale on edits unrelated to the decision they guard,
facing a corpus whose most-cited files are its most-edited ones, we decided that **a locator may carry a
scope between its path and its pin, naming one symbol or one line range, and that scope is found by a
resolver the repository declares in config**, to achieve pins that go stale only when the cited item
changes, accepting that resolvers are approximate and must be kept right by the repository.

**1. One grammar.** A locator is

```
locator := category ":" path [ "#" scope ] [ "@" anchor ]
scope   := "L" line [ "-L" line ]  |  symbol
anchor  := "sha256:" 64 lowercase hex  |  7 to 40 lowercase hex (a commit)
```

`category` is `spec`, `code` or `test` as today, and it alone decides the evidence tier, as today. A
locator without `#` and `@` is unchanged, and so is `path@sha256:<hex>` (`ADR-67`). Examples:
`code:rust/crates/urzua-core/src/config.rs#RuleSettings@sha256:<hex>`,
`code:.github/workflows/ci.yml#L10-L40@sha256:<hex>`.

**2. Where `MILE-84` fits.** The `sha256:` anchor is a content pin. The commit anchor (`@<commit>`) is
reserved in the grammar for `MILE-84`: it reads the scope as of that commit, a pointer to a site that
survives a refactor, not a content hash. `MILE-84`'s candidate `path@<short-sha>:<line>` is therefore
written `path#L<line>@<short-sha>`. The two anchors are mutually exclusive on one locator. This ADR decides
the slots and the order (path, scope, anchor); it builds only the content pin. `MILE-84` decides what a
commit anchor verifies in its own ADR, and that ADR inherits this grammar rather than choosing another.
The pin parser stays what `ADR-67` made it: only `@sha256:` plus exactly 64 lowercase hex is a pin, and
any other text stays in the path, so a malformed pin is a locator that names nothing, reported by
`embodiment.locator-exists`, never an unpinned one.

**3. The hashed bytes.** The scope resolves to a contiguous run of whole lines. The pin is the sha256 of
those lines exactly as stored, joined by `\n` (a CRLF file is hashed after line-ending normalisation, so
the same text hashes the same on every platform). Nothing else is normalised: not whitespace, not
comments. A reformat of the cited item is drift. That is the conservative choice: a pin may be re-set by
hand but must never pass a change it should have reported.

**4. A line range** `L<a>-L<b>` (inclusive, 1-based; `L<a>` alone is one line) needs no resolver and works
on any text file. It is exact and also brittle: lines inserted above it shift the content out of the range
and the pin goes stale. It is for files where a symbol cannot be named (a workflow, a config block) and
the span is stable. It does not claim to fix the edits-above case; the commit anchor of `MILE-84` is where
that is addressed.

**5. A symbol is found by a declared resolver, not a parser.** The engine ships no language knowledge.
Config declares resolvers as structured data (`ADR-64`), each naming the files it covers by glob, how a
symbol's first line is recognised, and how its last line is found, the latter from a closed set the engine
implements generically: `brace_balanced` (balanced `{}` from the first line), `indent` (until a line
indented no deeper than the first), `heading` (until the next heading of equal or higher level), `blank_line`
(until the next blank line). The start recogniser is a declared pattern in which the symbol is substituted
and escaped, the same `pattern` form the config already uses for placeholder vocabularies. Exact key names
are fixed in `SPEC-3` when this is built. A repository that wants Rust symbols declares a Rust resolver; a
default set may be scaffolded by `urzua init` as ordinary config the repository then owns, never compiled
into the engine. A resolver is approximate by nature (a pattern, not a grammar) and the ADR says so
rather than hiding it: it finds the item a human meant in the repository's own code, and where it cannot,
point 6 applies.

**6. A scope that stops resolving is reported, never passed.** `embodiment.locator-exists` reports, with
the locator and the reason, each of: the file is missing (as today); no resolver covers the file, so the
scoped form is **unsupported for that file type** (a symbol scope only; a line range always works); the
symbol matches nothing (renamed, moved or deleted); the symbol matches more than once (ambiguous: never
first-match-wins); the range lies beyond the end of the file; a resolver is malformed (a load error at
config validation). A locator reported this way is not also evaluated for drift, so one cause produces one
finding. `urzua fix --rebaseline` re-pins a scoped pin exactly as it does a file-level one, but refuses to
re-pin one that does not resolve and lists it as failed with the reason, because re-hashing "whatever is at
the range now" would turn a deleted symbol into a silent pass. A rename is repaired by editing the scope,
which is a visible diff in the record.

**7. Unpinned scoped locators and the rest of the evidence model.** A scoped locator with no pin is
existence-checked at its scope (point 6) and its drift falls back to the git-history heuristic of `ADR-32`
at file level, as an unpinned locator does today; the scope sharpens only a pin. `embodiment.locator-promotion-
candidate` keys on path and scope, so two decisions citing different symbols of one file are not a shared
claim. Embodiment tiers, the grace window, `claim_max_age_days` and back-pointers are unchanged.

## Reversibility

Hard to reverse once records use it: the grammar is written into record headers, many of them in Accepted,
frozen ADRs, so changing it later means a migration of every scoped locator. That is why the slots (scope,
then anchor) are decided now and the resolver config, which can change freely, is not. Everything else is
additive: a repository that writes no scoped locator and declares no resolver sees no change, and a
file-level pin keeps its `ADR-67` meaning. Walking the feature back means re-pinning each scoped locator
at file level, which is the state the corpus is in today.

## Consequences

- The 11 needless re-pins of the review would not have occurred, and the three real divergences would
  still have been reported, if each pin had been scoped to the item its decision is about.
- A pin is only as good as its resolver. A too-loose start pattern pins the wrong item silently; the
  mitigations are point 6 (ambiguity is reported) and the review of a resolver as config.
- Whitespace-only edits inside a pinned item are drift (point 3). That is noise of the same kind this ADR
  removes elsewhere; it is accepted for now so that no pin can pass a change it should have caught.
- Line-range pins on a file that grows above them will go stale; they are a fallback, not the main use.
- A symbol rename is a finding the author must fix by hand; there is no rename tracking.
- Accepting this ADR amends `ADR-67` (its pin syntax gains the scope slot) and puts a constraint on
  `MILE-84` (the commit anchor uses the anchor slot). Neither record is changed by drafting this one.
- Building it follows `MILE-185`: the grammar and `embodiment.locator-exists` reasons, hashing, resolver
  config in `SPEC-3`, `--rebaseline` in `SPEC-8`, `SPEC-2` for the rule, then narrowing this repository's
  pins on `config.rs`, `check_integration.rs`, `main.rs`, `embodiment.rs` and `.urzua/config.yaml`.

### Open questions for the maintainer

1. Qualified symbols. Should a scope accept a qualified name (`Config::load`, `Type.method`), and if so how
   does a resolver map it, or is one bare identifier enough for the first build?
2. Whitespace. Is exact-bytes hashing (point 3) right, or should trailing whitespace and line endings be
   normalised, accepting a weaker pin for fewer false drifts?
3. Default resolvers. Should `urzua init` scaffold resolvers for common languages as config the
   repository owns (point 5), or should the engine ship none at all and leave them to the repository?
4. Line ranges. Given their brittleness (point 4), should the first build ship only symbol scopes, leaving
   ranges until the commit anchor of `MILE-84` exists?
5. Anchor order. Is `path#L<line>@<commit>` acceptable as the spelling of `MILE-84`'s candidate, and may
   this ADR bind `MILE-84` to it, or must `MILE-84` be decided first?
6. File names containing `#`. The last `#` followed by a valid scope is read as the scope; a path that ends
   so cannot be cited in the scoped form. Acceptable?

## References

- `ADR-67`: file-level pins, the grace window and the claim age; this ADR extends its pin syntax.
- `ADR-32`: git-history drift, still the fallback for an unpinned locator.
- `ADR-18`: categorized locators; the evidence tier is unchanged.
- `ADR-64`: structured in, structured out.
- `RFC-5`: defines a locator as a file or glob with no sub-file form.
- `MILE-185`: the build this ADR gates; `MILE-84`: line- and commit-anchored locators, which share this grammar.
- `MILE-181`: attributed re-baselining.
- `SPEC-2`, `SPEC-3`, `SPEC-8`: the specs the build documents this in.
