---
Status: Fixed
Stable-Id: 01M1Y4SP0QDPAW46065XBT3T0M
Found-in: raised directly -- questioning why filenames are zero-padded to a fixed width at all, since a fixed width bounds the total count a type can ever hold
Realized-by: code:rust/crates/urzua-core/src/rules/shared.rs#record_id@sha256:63f5271f42575e02888c30200a025d2d4cbd6ce2bc1330c99221dee0595d881c, code:rust/crates/urzua-core/src/values.rs#normalize@sha256:6fb810ffa3b6a73bb0f21f090f598c1cfab29f83f5786a42a7641838ead3bb75, test:rust/crates/urzua-core/src/values.rs#record_id_compares_by_numeric_value@sha256:2e7b3ad606e780cf911ea4ca968583e6965edd3139e656283974644fb202a570, test:rust/crates/urzua-core/src/rules/identity.rs#a_five_digit_filename_still_resolves_bug_0002_observed_failing@sha256:c6c211b53673fcc7a3860427bc88fe9317bdf168163a1328c15333a19e59dadd, test:rust/crates/urzua-core/src/rules/identity.rs#a_reference_resolves_regardless_of_zero_padding@sha256:c7aba9a539e154df6c5f94144717e14a7d84db3e1b3d68486813d0f3c19ee08b
---
# 2 — record_id hardcoded a 4-digit numeric prefix, bounding every type at 9999 records

## What was wrong

`record_id()` rejected any filename whose numeric prefix wasn't exactly 4 digits
(`number.len() != 4`). Past 9999 records of one type, a 5-digit filename would fail to produce an
ID at all -- every `Implements`/`Derives-from`/`Supersedes` reference to it would silently report as
non-resolving, with no warning that the cause was digit count rather than a genuinely broken
reference. Nowhere in ADR-3 (the stable-identifier decision) is a 4-digit width discussed or
justified -- it was never a deliberate choice, just an unexamined convention that happened to match
common ADR-numbering style.

A second, subtler problem sat underneath the obvious one: `record_id()` derives an ID from the raw
filename digit string, preserving whatever padding the filename happens to have, and matching was
exact string equality. Simply widening the digit-count check would not have been sufficient on its
own -- a filename `34-x.md` (unpadded) and a hand-typed reference `ADR-34` (padded) would still
fail to match each other as the same record, just with a different, equally silent failure mode.

## Why nothing caught it

No existing test constructed a filename with anything but exactly 4 digits, and no test constructed
a reference with different padding than its target's filename -- both gaps in coverage, not just one.

## Fix

Two changes, not one: `record_id()` now accepts any non-empty all-digit numeric prefix (no fixed
width). Matching in `pointer_resolution` and `supersession_reciprocity` now compares by numeric
value via a new `normalize_id` helper (strips leading zeros before comparing), not by exact string
equality -- so `ADR-34` and `ADR-34` resolve identically regardless of which one a filename or a
hand-typed reference happens to use. No existing filename needs to change; the fix is purely in how
two representations of the same number are recognized as equal.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `record_id`, `normalize_id`, `pointer_resolution`,
  `supersession_reciprocity`.
- BUG-1 -- found and fixed in the same session, same underlying discipline: exercise a case
  nothing had exercised before, rather than trusting an assumption nobody had checked.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-07 | Initial record: defect found, fixed, and verified same-day. | **structural** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 3 locators pinned, 0 file-level, 3 scoped. Evidence: rust/crates/urzua-core/src/rules/shared.rs#record_id@sha256:63f5271f4257, rust/crates/urzua-core/src/values.rs#normalize@sha256:6fb810ffa3b6, rust/crates/urzua-core/src/values.rs#record_id_compares_by_numeric_value@sha256:2e7b3ad606e7 Reason: Pinned the locators re-pointed in this review of PR 433 against the code as it is now: record_id in rules/shared.rs still accepts any non-empty numeric prefix, values.rs normalize still strips leading zeros so ADR-0034 and ADR-34 are one id (BUG-2), filename_title_consistency in rules/filename.rs still separates an unnumbered H1 from an absent one and reports the H1's own line (BUG-23, BUG-29), first_h1 in rules/filename.rs still ignores an H1 inside a fenced block (BUG-30), discovery.rs scope_to_requested_paths still narrows check to the requested paths (BUG-1), and doctor.rs DoctorReport is still the JSON doctor emits (BUG-4). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | `Realized-by` re-pointed at where the evidence now lives: `code:` and `test:` `rules.rs` replaced by `rules/shared.rs#record_id`, `values.rs#normalize` and `values.rs#record_id_compares_by_numeric_value`, each checked against the code it names. **Why:** review of PR 433: the fix is intact but moved. `record_id` (now in `rules/shared.rs`) still accepts any non-empty all-digit prefix with no fixed width, and the numeric comparison the record calls `normalize_id` is now `normalize` and the `RecordId` type in `values.rs`, which strips leading zeros so `ADR-0034` and `ADR-34` are one id. `rules.rs` is now a module root holding only shared test fixtures and no longer holds this fix; the two `identity.rs` regression tests were already pinned and still resolve. | **substantive** |
