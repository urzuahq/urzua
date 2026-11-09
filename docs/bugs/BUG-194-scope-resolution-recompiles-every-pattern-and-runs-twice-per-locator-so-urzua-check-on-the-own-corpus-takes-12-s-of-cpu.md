---
Stable-Id: 01M6Z31G358JHNG5Q0SZY2PY4M
Status: Fixed
Found-in: 'a full-release code review, round 5: urzua check on this repository took 45 s (debug) and 12 s of CPU (release)'
Realized-by: code:rust/crates/urzua-core/src/locator.rs, test:rust/crates/urzua-core/src/locator/tests.rs
---
# 194 — Scope resolution recompiles every pattern and runs twice per locator, so urzua check on the own corpus takes 12 s of CPU

## What was wrong

Every scope resolution built its regexes from scratch: `compile` built a new `Regex` from the resolver template on each resolution, `select_resolver` rebuilt the anchored glob regex of every resolver on every call, and the `leading` pattern was compiled per call. The same symbol scope in the same file was also resolved twice per locator in one run, once by `embodiment.locator-exists` (`locator_scope_defect`) and once by the drift judge (`pins::read_pin`), each scanning every line of the file, and `read_pin` cloned the whole file text first. On this repository's own corpus `urzua check` cost about 45 s in a debug build and 12 s of CPU in a release build, and every integration test that checks the own corpus inherited it. The cost grows with the number of scoped locators times the number of resolvers, so an adopter with many scoped pins pays it first.

## Why nothing caught it

Every test resolves a handful of scopes against a one-resolver configuration, where building a regex per call is invisible. Nothing measured a whole-corpus run, and the two callers each looked correct on its own: neither could see that the other had already resolved the same scope.

## References

Fixed in `rust/crates/urzua-core/src/locator.rs`: compiled patterns and glob regexes are kept per distinct pattern text, and a finished symbol resolution is kept under its whole key (text, resolver, scope), compared in full on a hit, so two configs or two file texts never share an entry. `pins::read_pin` no longer clones the file text. Output is unchanged: `check`, `check --base`, `audit`, `fix --rebaseline`, `reverify` and `explain --rule` print byte-identical JSON before and after. The tests in `rust/crates/urzua-core/src/locator/tests.rs` under "shared work between resolutions" hold the keying: a repeated resolution, two resolvers with one name, two texts and a failing `leading` pattern each give the answer they gave before. Timings are in the pull request.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-09 | Filed and fixed in one pass. **Why:** the round-5 code review measured `urzua check` on this repository at 12 s of CPU in release, almost all of it scope resolution; the fix changes no output. | **substantive** |
> | 2026-11-09 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `Realized-by computes to Verified` -- stated `Open` disagreed with the computed value `Fixed`. | **structural** |
