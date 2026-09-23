---
default: patch
---

Fixes a real nondeterminism defect found by review: a cross-type record-identifier collision's
first-seen-wins index slot depended on `HashMap` iteration order (randomized per process), so which
of two colliding records other rules resolved against could flip between runs of an unchanged
corpus. `Config::sorted_type_names()` is now the one shared, sorted way every `record_types`
iteration site gets its type list, replacing 11 independently hand-written `collect()`+`sort()`
call sites and 3 that iterated unsorted.

Also: a comment in `config.rs` narrating its own prior wording (rather than stating the current
invariant) rewritten to just state the invariant; an algebraic no-op in `new_record.rs`'s filename
parser simplified; `report.rs`'s `census`/`census_records` tallying logic de-duplicated (the latter
now implemented in terms of the former); and `check.rs`'s two near-identical symlink-containment
checks in `claim_paths`' walk consolidated into one shared `resolve_inside_repo` helper.

No adopter-facing behavior change: verified with the full test suite, clippy, `make ci`, and a
real-corpus `check` run reporting the same 70 findings before and after. The `HashMap`-ordering fix
was additionally verified empirically across 20 separate process invocations of the pre-fix binary
against a real collision fixture (13/20 resolved one way, 7/20 the other) versus 20/20 stable on the
fixed binary — the nondeterminism can't be captured as a classic single-process observed-failing
test, so this round's regression test instead asserts the fix's actual guarantee directly.

Files `BUG-129` (left `Open`, not rushed): `parse_realized_by` truncates a `Realized-by` locator path
at a literal comma before checking for the category delimiter, since the format splits on `,` before
`:`. Fixing it needs a delimiter/escaping decision this changeset doesn't make.

Two additional "duplication" findings from the same review were investigated and refuted: both
`scan_references`/`extract_references`'s reference-token check and `config.rs`'s rule-option
validation were already fully consolidated through a single shared function/table in the current
code — the review described an already-fixed prior state, not the code as it stands.
