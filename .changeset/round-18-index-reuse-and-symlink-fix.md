---
default: patch
---

Fixes `BUG-114`: `parse_record_filename`'s prefix check accepted only ASCII-uppercase letters, so a
digit-bearing type prefix (`V2`, `S3`) never matched, silently breaking `urzua new`'s duplicate-number
protection for such types. Digits are now allowed alongside uppercase letters in a prefix segment; the
number segment can't be confused with it, since it's already selected as the first all-digit segment.

Fixes `BUG-115`: `check.rs` called `compute_drifted_records` (which shells out to `git` per
`Realized-by` field) unconditionally, even when `embodiment.consistency` — its only consumer — is
configured `Off`. It now runs only when that rule is enabled.

Fixes `BUG-116`: `identity.collision` rebuilt the shared normalized-record index independently instead
of reusing the one `check.rs` already builds for five other rules, doing the same O(corpus) traversal
twice per run. `build_index_reporting_collisions`/`IdentifierCollision` are now `pub`, and
`identity_collision` takes the pre-computed collisions list as a parameter.

Fixes `BUG-117`: the `claim_paths` symlink-ancestor guard (`prefix_canonical.starts_with(&resolved)`)
was reflexive, rejecting a symlink that resolves to exactly the declared prefix root as if it were a
genuine ancestor, even though `visited` already dedupes it safely. Now requires
`resolved != prefix_canonical` in addition.

No adopter-facing behavior changes beyond the four fixes above: verified with the full test suite,
clippy, `make ci`, and a real-corpus `check` run reporting the same findings before and after. One
other candidate from this review pass (`header_field_case_mismatch` vs. `near_miss`'s inline
near-miss logic) was investigated and found not to be a real duplication risk — both already share
`FieldName`'s single `Ord` impl, so they cannot diverge — and one (a temporal-language comment quote)
was refuted as a hallucinated citation not present in the file.
