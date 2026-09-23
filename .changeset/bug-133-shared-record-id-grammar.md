---
default: patch
---

`is_record_reference` (prose/pointer-field record mentions) and `parse_record_filename` (filename
parsing) each independently re-implemented the same two "is this a valid record identifier segment"
predicates -- byte-for-byte identical code with no enforcement that they stay in sync. This duplication
is exactly how they drifted apart twice already (a hyphenated prefix, then a digit-bearing prefix), each
time silently breaking every reference to a record of that shape until someone noticed and filed a bug.

The two predicates are now one definition each (`is_digit_segment`, `is_prefix_segment` in
`new_record.rs`), shared by both recognizers. No behavior change; a regression test pins their agreement
so a future edit to one can't silently re-open the drift.
