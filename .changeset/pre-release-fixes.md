---
default: major
---

Implements `ADR-50`, previously Accepted and unbuilt: `header_shape: "none"` declares a type whose
records carry no header block at all. `header.required-fields` skips such a type instead of reporting
every record as missing a header region; a new rule, `config.header-none-has-no-required-fields`,
reports the self-contradiction of declaring one anyway; `header.deprecated-shape` is now an explicit
enumeration of the two actually-deprecated shapes instead of a negative test that would have flagged
`none` on sight; `urzua new` refuses to generate a record for a `none`-shaped type rather than writing
one with no identity. Fixes `BUG-98`.

`init` now warns when its proposed config leaves `urzua audit`'s entire rule set undeclared — the
ordinary outcome for a corpus with no filename type prefix, since both of `audit`'s rules are
identity-dependent. `audit`'s own exit-code behavior is unchanged (a run in which nothing ran still
reports `not-run`); this only tells the adopter why before they hit it. Fixes the disclosure half of
`BUG-99`; the root cause (`audit` owning a hardcoded rule subset) stays tracked by `RFC-38`.

**Breaking: `claim.status-agreement`, `pointer.target-status`, and `narrative-field.stale` now only
read a target record's `Status` field when that record's own type declares `Status` in
`required_fields` or `known_fields`** — the same rule `header.field-case-mismatch` already applies to
every other field. A type that never declares `Status` previously had it checked anyway, hardcoded,
which meant a mis-cased `status:` on such a type went both unenforced and undiagnosed; it also means a
type relying on the old unconditional check now needs to declare `Status` explicitly to keep these
three rules' coverage. Fixes `BUG-109`, decided by `ADR-60`.

`read_claim_files` now reads through `urzua_io::read_to_string`, the same chokepoint every other record
read already uses, instead of calling `std::fs::read_to_string` directly.

Three independent, hand-maintained lists describing which rules take which config options (whether an
option is declared on the wrong rule, whether a rule requires options to load at all, and what a
missing required option's consequence is) collapse into one table in `urzua-core`, so a future
option-taking rule is one entry instead of three edits that could disagree. `init`'s notion of which
rules depend on filename identity moves from `urzua-cli` into the same crate as the rules themselves.

No adopter-facing behavior changes beyond what's described above: verified with the full test suite,
`make ci`, and a real-corpus `check` run reporting the same findings before and after.
