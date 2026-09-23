---
default: patch
---

Fixes three real defects found by review:

- `RecordId::normalize` split at the first hyphen, not the last, so a multi-segment type prefix
  (`DOC-ADR`, already a supported shape per `BUG-111`/`BUG-114`) never matched a zero-padded and
  unpadded reference to the same record as one id.
- `is_record_reference` still rejected a digit-bearing prefix segment (e.g. a type declaring
  `prefix: "V2"`), even though `BUG-114` already loosened `parse_record_filename` to accept one — a
  mention of such a record in prose or a pointer field never resolved.
- `check.rs`'s `claim_paths` pre-flight validation re-derived its own symlink-containment check
  instead of using the `resolve_inside_repo` helper defined specifically so this check has one place
  to land; both now share a `path_is_inside_repo` predicate.

No adopter-facing behavior change beyond correctness: verified with the full test suite (each fix has
a planted-violation test, observed failing before and passing after), clippy, `make ci`, and a
real-corpus `check` run reporting the same 70 findings before and after.

Three more findings from the same review were investigated and refuted or filed rather than rushed:
`records_read_by_any_rule`'s report-scope vs. each rule's whole-corpus population is deliberate
(`BUG-67`); `init`'s independence from `docs/` is deliberate (`BUG-36`). `BUG-130`
(`duplicate_keys`'s exact-match reach), `BUG-131` (the identity index's undisclosed first-seen-wins
resolution when `identity.collision` is off), and `BUG-132` (`init`'s directory-disambiguation missing
a second collision check) are filed, left `Open` — each needs a real design decision, not a rushed fix.
