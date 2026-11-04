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
  `rust/crates/urzua-core/src/config.rs` 35, `rust/crates/urzua-cli/tests/check_integration.rs` 15,
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
| B. A scope between path and pin: `path#<symbol>` or `path#L<a>-L<b>`, symbols found by resolvers declared in config (chosen) | Pin goes stale only when the cited item changes; no language list in the engine; one grammar that can also carry `MILE-84`'s commit anchor; file-level locators unchanged | Each resolver is config an adopter must write and keep right; a regex resolver is approximate; a line range moves when lines are added above it; a pin hashes the span the resolver finds, so a resolver is part of the contract |
| C. Markers inside the cited file (`// urzua:begin ADR-57` ... `// urzua:end`) | Exact span, survives edits above it | Edits the cited file for the record's sake; impossible in generated, vendored or comment-less files (`MILE-84` names these as the reason for locators); couples code to record ids |
| D. Parse each language (tree-sitter or similar) and hash the syntax node | Precise; whitespace and comment changes do not matter | A hardcoded list of languages and grammars in the engine, against the founding claim; a heavy dependency and a per-language upgrade burden |
| E. Pins as a separate header field (`ADR-67` option C) | Room for per-locator metadata | A schema change to `Realized-by`, which `ADR-18` kept flat; `ADR-67` already declined it |

## Decision

In the context of file-level evidence pins that go stale on edits unrelated to the decision they guard,
facing a corpus whose most-cited files are its most-edited ones, we decided that **a locator may carry a
scope between its path and its pin, naming one symbol or one line range, and that scope is found by a
resolver the repository declares in config**, to achieve pins that go stale only when the cited item
changes, accepting that resolvers are approximate and that the grammar and the end kinds are a contract the
corpus depends on.

**1. One grammar and one parse order.** A locator is

```
locator := category ":" path [ "#" scope ] [ "@" anchor ]
scope   := range | symbol
range   := "L" N [ "-L" M ]      N and M decimal, no leading zero, 1 <= N <= M
symbol  := seg ( "::" seg )*     seg := [A-Za-z_] [A-Za-z0-9_.-]*
anchor  := "sha256:" 64 lowercase hex  |  7 to 64 lowercase hex (a commit)
```

`category` is `spec`, `code` or `test` as today and alone decides the evidence tier. Text that matches
`range` is always a range, and a symbol is never spelled to look like one (`L10` is a range). A symbol
contains none of `/`, `@`, `#`, `,` or whitespace and does not start with a digit or `.`. Qualified names
(`::`) are in the grammar from the first build, so the irreversible part is decided once. 64 hex is allowed
for a commit because a repository using the SHA-256 object format has such commits; the `sha256:` prefix
disambiguates.

The parser takes, in order: (1) a trailing `@sha256:` plus exactly 64 lowercase hex, exactly as `ADR-67`
does; (2) otherwise a trailing `@` plus 7 to 64 lowercase hex, which is the reserved commit anchor (point
2); (3) in what remains, the text after the **last** `#`, as the scope only if it matches `scope`, and
otherwise the whole text is the path. `L0`, `L010` and `L40-L10` do not match `range` and, being
digit-leading or not symbols, are malformed. A locator carrying both anchors is malformed. A path ending in
`#word` that matches `symbol` therefore reads as a scope, and such a file cannot be cited even at file level;
no escape syntax is given. The corpus has no `#` in any `Realized-by` value today, so nothing regresses, and a
planted test covers it. A glob cannot carry a scope (`src/**#Foo` is malformed). A locator whose text
contains `#` or `@sha256:` and fails this grammar is reported by `embodiment.locator-exists` as a malformed
scope or pin naming the grammar, never as a path missing from the tree.

Examples: `code:rust/crates/urzua-core/src/config.rs#RuleSetting@sha256:<hex>`,
`code:.github/workflows/ci.yml#L10-L40@sha256:<hex>`, `code:rust/crates/urzua-core/src/config.rs#RuleSetting::default@sha256:<hex>`.

**2. Where `MILE-84` fits.** The `sha256:` anchor is a content pin. The commit anchor (`@<commit>`) is
reserved for `MILE-84`: it reads the scope as of that commit, a pointer to a site that survives a refactor,
not a content hash. `MILE-84`'s candidate `path@<short-sha>:<line>` is therefore written
`path#L<line>@<short-sha>`. This ADR binds the slot order (path, scope, anchor) and the anchor token shape
(hex, 7 to 64), and nothing else; what a commit anchor verifies stays in `MILE-84`'s ADR. It builds only the
content pin.

**3. The hashed bytes.** The scope resolves to a contiguous span of whole lines. The pinned bytes are the
span's lines, each with at most one trailing `\r` removed, joined by `\n`, with no trailing newline, read as
UTF-8 (a leading U+FEFF on line 1 is dropped). A file that is not valid UTF-8 cannot carry a scoped pin and
is reported. Nothing else is normalised: not whitespace, not comments, so a reformat of the item is drift;
the engine cannot know whether whitespace is semantic (Python, YAML, a Makefile). A file-level pin is
unchanged and still hashes the raw bytes of the file, so the same text has a platform-stable scoped hash and
a line-ending-dependent file-level hash, and a scoped pin over every line of a file is not equal to that
file's file-level pin (changing the file-level hash would stale every existing pin). `SPEC-8` gives a shell
recipe that reproduces a scoped hash from a given span. A later per-resolver normalisation would be opt-in
and counts as a resolver edit (point 8).

**4. A line range** needs no resolver and works on any text file. It is exact and brittle: lines inserted
above it shift the content out of the range and the pin goes stale, and the finding says the text may have
moved. It is for files where a symbol cannot be named (a workflow, a config block) and is shipped in the
first build, because point 6's answer for a file type with no resolver depends on it. A symbol scope
survives a move of the item within its file, and a file rename passes once the locator path is edited and
the span is unchanged. For generated files a symbol survives reordering and a range does not; cite the
generator's input where there is one.

**5. A symbol is found by a declared resolver, not a parser.** The engine ships no language knowledge.
Config declares resolvers as structured data (`ADR-64`), each naming the files it covers by glob, how a
symbol's first line is recognised, and how its last line is found from a closed set of **end kinds** the
engine implements generically: `brace_balanced` (balanced `{}` from the first line), `indent` (until a line
indented no deeper than the first), `heading` (until the next heading of equal or higher level),
`blank_line` (until the next blank line). The start recogniser is a declared pattern in which the symbol is
substituted after regular-expression escaping (the symbol arrives from record text). Optional parts: a
`leading` pattern extends the span upward over contiguous matching lines (attributes, doc comments), and
without it leading lines are not pinned; a `member` pattern supports one container level, so `A::b` finds
`A` uniquely with `start` and then `b` uniquely with `member` inside A's span, which a repository needs to
address methods and `impl` blocks. Exact key names are fixed in `SPEC-3` when this is built. Validation is
at config load (`ADR-64` clause 5): two resolvers whose globs cover one file, a start pattern without the
symbol placeholder, or a malformed pattern is a load error, never first-wins. A default set may be
scaffolded by `urzua init` as ordinary config the repository then owns, never compiled in, only for file
types the corpus actually cites, each with test vectors (a planted ambiguous and a planted early-end case).

**6. A scope that stops resolving is reported, never passed (while `embodiment.locator-exists` is
enabled, as with a missing file today).** The rule reports, with the locator and the reason: the file is
missing (as today); the file is not valid UTF-8; no resolver covers the file, so a symbol scope is
unsupported for that file type (a line range always works); the symbol matches nothing (renamed, moved out
of the file or deleted); the symbol matches more than once (ambiguous: never first-match-wins); the end is
not found (never balanced before end of file); the range lies beyond the end of the file. A locator reported
this way is not also evaluated for drift, so one cause produces one finding. `urzua fix --rebaseline`
re-pins a scoped pin as it does a file-level one but refuses one that does not resolve and lists it as a
failed pin with its reason, per pin and not per record, because re-hashing "whatever is at the range now"
would turn a deleted symbol into a silent pass. A rename is repaired by editing the scope, a visible diff
in the record. The engine cannot detect an early end (a `}` inside a string ends a `brace_balanced` span
early and the rest is unpinned), so point 7 puts the resolved span in front of the reviewer.

**7. One authoring route, and the resolved span is always shown.** A person cannot reproduce a regex plus an
end kind by hand, so the supported way to obtain a first hash is `urzua explain <path>#<scope>`: it prints
the resolved first and last line, and the hash, ready to paste after
`@sha256:`. `urzua fix --rebaseline` lists the resolved span beside the recorded and current hash, so the
reviewer diffs exactly those lines and sees that the resolver found the item the decision is about. A
bogus placeholder hash re-baselined later would record a re-verification of something never verified, and
is not a supported route. `explain <path>` also reports the scope of each record that cites the path.

**8. Drift for a scoped locator.** For a scoped pin, freshness is not judged from the cited file's last
commit, because that commit says nothing about the scope: a scoped pin that no longer matches is drift
immediately, unless the record declares an audit date (`drift_audit_field`), in which case the window
counts from that date as for a file-level pin. (Using `git log -L` for the span's last commit is correct but
ties drift to git's line tracking and costs a subprocess per pin; it is not taken in the first build.) A
scope **without a pin** has no effect on drift, so `embodiment.locator-exists` reports it as "scope given
without a pin: drift is judged on the whole file", at the rule's severity; the scope is still
existence-checked, so an unpinned scoped locator is never a silent no-op. Comparing the scope's content at
the commit that last touched the `Realized-by` line is real scope-level drift without a pin, but it is a
second mechanism and belongs to `MILE-84`'s commit-anchor ADR. `embodiment.locator-promotion-candidate`
keys on path and scope, so two decisions citing different symbols of one file are not a shared claim.
Embodiment tiers, `claim_max_age_days` and back-pointers are unchanged.

**9. Shape in the code (`SPEC-22`).** Resolution is one pure function over text,
`resolve_scope(text, resolver, scope) -> Result<Span, ScopeError>`, in `urzua-core`; discovery reads each
file once per run (memoised, as `BUG-135` did for git) and the existence, drift and re-baseline call
sites share one outcome type in place of today's `bool` and `Option`. A locator is parsed once into
`Locator { path, scope, anchor }`; every consumer other than drift, existence and re-baselining (`explain`,
`check --base`, status, promotion) uses `.path`, so a record that cites `config.rs#RuleSetting` still
answers for `config.rs`. The re-pin edit keeps the scope in the spelling as written.

**10. Resolvers are part of the contract.** See Reversibility.

## Reversibility

Hard to reverse once records use it: the grammar is written into record headers, many of them in Accepted,
frozen ADRs, so changing it later means a migration of every scoped locator. The grammar and the behaviour of
each end kind are the contract. A resolver's patterns are config, but editing one changes the bytes under
every pin that uses it, so a resolver edit is a corpus-wide re-baseline (an attributed row on each record)
and `urzua check` reports it as such. End kinds are append-only: the behaviour of a shipped kind never
changes (a fix ships as a new kind name), and `SPEC-3` states each kind normatively with test vectors.
Everything else is additive: a repository that writes no scoped locator and declares no resolver sees no
change, and a file-level pin keeps its `ADR-67` meaning. Walking the feature back means re-pinning each
scoped locator at file level, the state the corpus is in today.

## Consequences

- The 11 needless re-pins of the review would not have occurred, and the three real divergences would still
  have been reported, if each pin had been scoped to the item its decision is about.
- A pin is only as good as its resolver. A too-loose start pattern pins the wrong item; ambiguity is
  reported, an early end is not detectable, and `brace_balanced` is blind to braces in strings and comments,
  `indent` to odd blank lines and tabs, `heading` to `#` lines inside fenced code. Test fixtures that embed
  source as strings can produce a second match, so ambiguity occurs in files that look unambiguous.
- Adding a second item of the same name anywhere in the file turns a passing pin into an ambiguity finding,
  the unrelated-edit noise this ADR removes. Strict is kept for the first build because it is what the ADR
  promises. A recorded alternative: when a pin exists, resolve ambiguity by content (pass if exactly one
  candidate hashes to the pin), tolerant of moves without being first-match-wins.
- Whitespace-only edits inside a pinned item are drift (point 3).
- Line-range pins on a file that grows above them go stale; they are the fallback (point 4).
- A symbol rename is a finding the author fixes by hand; there is no rename tracking.
- `--force` re-baselines every stale record under one `--reason`; with scoped pins that is smaller but still a
  bulk attestation.
- The bare form cannot name every item (the struct `RuleSetting` is followed by two `impl` blocks for the
  same name, and `new` or `default` recur in every impl), which is why qualified names and the `member`
  pattern are in the first build. If the maintainer prefers a smaller first build, `MILE-185`'s Done means is
  amended to the items a bare name can address and `::` stays reserved.
- Accepting this ADR sets housekeeping for the same change: a dated `## Amendment` to `ADR-67` (its pin
  syntax gains the scope slot), a revision row on `MILE-84` (the anchor slot is taken), `MILE-185` set to
  `Implements: ADR-74`, rows in `SPEC-2`, `SPEC-3` and `SPEC-8`, and a re-check of the five files `MILE-185`
  names against what the first build can address.
- Building it follows `MILE-185`: grammar, the `embodiment.locator-exists` reasons, hashing, resolver
  config in `SPEC-3`, `--rebaseline` in `SPEC-8`, `SPEC-2` for the rule; then narrowing this repository's pins
  on `config.rs`, `check_integration.rs`, `main.rs`, `embodiment.rs` and `.urzua/config.yaml`, tested with
  the default grace window, not only with `drift_grace_days: 0`.

### Questions answered in review, for the maintainer to confirm

1. Qualified symbols: `::` is in the grammar now; the first build supports one container level through
   `member`.
2. Whitespace: exact bytes with only the line-ending, BOM and final-newline normalisation of point 3.
3. Default resolvers: scaffolded by `urzua init` as repository-owned config, only for file types the corpus
   cites, with test vectors.
4. Line ranges: in the first build.
5. Anchor order: bound (path, scope, anchor; hex 7 to 64); what a commit anchor verifies stays `MILE-84`'s.
6. A `#` in a file name: accepted, with the rule of point 1 and no escape syntax.

## References

- `ADR-67`: file-level pins, the grace window and the claim age; this ADR extends its pin syntax.
- `ADR-32`: git-history drift, still the fallback for an unpinned locator.
- `ADR-18`: categorized locators; the evidence tier is unchanged.
- `ADR-64`: structured in, structured out.
- `RFC-5`: defines a locator as a file or glob with no sub-file form.
- `MILE-185`: the build this ADR gates; `MILE-84`: line- and commit-anchored locators, which share this grammar.
- `MILE-181`: attributed re-baselining.
- `SPEC-2`, `SPEC-3`, `SPEC-8`, `SPEC-22`: the specs the build documents this in and follows.
