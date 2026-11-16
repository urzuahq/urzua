---
Stable-Id: 01M7GY90WHXBXBR6YZ3A793HE5
Status: InProgress
Phase: '1'
Track: embodiment-model
Blocked-on: null
---
# 284 — Re-pins stop making every pair of code pull requests conflict

## What

Change how a re-pin is recorded or merged so that two pull requests that each re-pin the same records can both merge without hand-resolving every record.

## Why

On 2026-11-15 and 2026-11-16 one pull request re-pinned about 85 records and another about 100; each merge to the default branch made the other conflict in dozens of records, and the second had to be rebuilt from scratch three times. Whole-file pins on a few central files mean any change to them touches a large share of the corpus.

## Done means

- Two branches that each change a pinned file and re-pin the same record merge without a conflict in that record, or one documented command resolves all such conflicts and re-pins, shown by a test with two such branches. — evidence: test:rust/crates/urzua-cli/tests/fix_conflicts_integration.rs#a_record_both_merged_sides_re_pinned_is_resolved_on_merge_observed_failing@sha256:729e1d29142afe808a2a65e6a93c5b5977e8dd182a65bda162a0e5c8230c3661
- The number of records pinned to a whole central file falls, by narrower pins or back-pointers, and the count before and after is recorded.
- The contributor documentation says what to do when a branch's re-pins conflict.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-16 | Initial record, `Status: Planned`. **Why:** Found while landing parallel pull requests on 2026-11-15 and 2026-11-16. | **structural** |
> | 2026-11-16 | Evidence added to the first Done-means item: `urzua fix --conflicts` resolves a record that two branches each re-pinned, shown by a test that merges and rebases two such branches. Decided by the maintainer on 2026-11-16: asked "Should urzua get a command that resolves those conflicts itself: keep both pull requests' log rows, recompute the pin from the merged code, and add a row saying the pin was recomputed on merge? It would refuse any record that conflicts in its real content.", answered "Yes, add the command"; then asked "Should fixing conflicts after a merge be one command under `fix`? It would do both jobs in one run: give a record whose number is now taken the next free number (renaming the file and rewriting every reference to it), and resolve records that conflict only in their evidence pins. Anything it can't safely fix, it lists and leaves alone.", answered "`urzua fix --conflicts`". The other items are not delivered here and `Status` is unchanged. | **structural** |
> | 2026-11-16 | Review correction: eleven records whose claims are kept true by a rule running gained that rule's entry as a further scoped locator (MILE-7, 27, 78, 91, 98, 153, 154, 155, 162, 163, 167). After: 5 whole-file locators in 2 records, 236 scoped locators in 84 records. A comment line under `rules:` still makes 5 records stale; switching any of the added rules off makes the records that rest on it stale. **Why:** The first narrowing named only the record-type declaration for these claims. | **substantive** |
> | 2026-11-16 | Tool-authored (by beauwilliams, identity: gh-verified, actor: agent): recomputed `Status` from `the Done means items (1 of 3 items resolved) computes to Verified` -- stated `Planned` disagreed with the computed value `InProgress`. | **structural** |
> | 2026-11-16 | Whole-file pins on `.urzua/config.yaml` narrowed to key paths. Before: 114 whole-file locators in 55 records, 74 scoped locators in 32 records. After: 5 whole-file locators in 2 records (MILE-177, MILE-178, whose claims rest on the whole config), 209 scoped locators in 84 records. A one-line change under `rules:` made 55 records stale before and 5 after (the 2 whole-file records and the 3 pinned to `rules` as a whole). The Done means item on whole central files is not marked: other central files are not yet narrowed. **Why:** Measured for the second Done means item. | **substantive** |
