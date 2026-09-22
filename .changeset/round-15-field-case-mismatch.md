---
default: major
---

Fixes two opposite-direction defects in how a declared field's value is read when its exact-cased key
is missing (`ADR-57`): `claim.status-agreement` and `pointer.target-status` used to fabricate a
sentinel that turned every claim or pointer citing a mis-cased `Status` field into a false blocking
finding; the embodiment rules used to silently drop a mis-cased `Realized-by`/`Embodiment`, hiding a
real inconsistency. Both now skip the field (unjudged, not judged-and-wrong), and a new rule,
`header.field-case-mismatch`, is the one place that reports a declared field written under a different
case (`ADR-58`). Also fixes a duplicate record-number bug in `urzua new` (`0013-01-15-slug.md` used to
parse as a date and skip record 13, which could then be reissued), consolidates three previously
independently-computed "record-shaped file" definitions into one, and consolidates the `(record,
field)` candidate-slot construction duplicated across five rules into two shared helpers.

Implements `RFC-39`/`ADR-59`: `FieldName` and `RecordId` are now real types whose `Eq`/`Hash` encode
their comparison rule (exact for field names per `ADR-57`, numeric-normalized for record identifiers
per `BUG-2`), replacing bare `String` at the six field-name comparison sites and the shared record
index. `RecordTypeConfig`'s own fields stay `Vec<String>` — the type boundary sits at
`declared_fields()` and the header's comparison methods, not at config deserialization, since that is
where every comparison bug in this family actually lived. The remaining ten rule functions that took a
bare `HashMap` projection of configuration now take `&Config` directly, matching the fifteen that
already did. Two enums that predate `ADR-53`'s move of status vocabulary to the adopter
(`urzua_core::Status`, `urzua_core::Embodiment`) are removed; neither was ever constructed.

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
clippy, `make ci`, and a real-corpus `check` run reporting the same findings before and after each
phase of this work landed.
