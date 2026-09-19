---
Stable-Id: 01M2WCN8QRXB4EN7779QV4X19K
Status: Open
Found-in: "Round 6 of the 0.4.0 release review"
Regression-test: "rust/crates/urzua-cli/tests/check_integration.rs::a_claim_path_prefix_is_read_to_any_depth"
---
# 63 — claim_paths is a prefix but is read one level deep so a nested layout goes inert

## What was wrong

`claim_paths` is documented as a list of path *prefixes* to scan for claims, and validated by testing
that each one is a directory. It is then read with a single non-recursive directory listing filtered
to `.md`.

A prefix whose claims live one level down -- `docs/changes/2026-09/0042-fix.md` under a declared
prefix of `docs/changes` -- passes the directory check, yields only a subdirectory entry, which the
extension filter discards. `claim.status-agreement` receives an empty claim list and reports
`records_examined: 0, status: ran` with no findings: indistinguishable from a corpus where every claim
agrees.

`BUG-56` added the guard that stops this rule going inert when `claim_paths` is missing or misspelled.
A prefix that is real, spelled correctly, and simply deeper than one level walks through the same
door the guard was built to close.

This repository's own `claim_paths` is `.changeset`, which is flat, so the corpus here is unaffected.

## Why nothing caught it

`BUG-56`'s guard was written against the two ways a prefix can fail to name a directory. The third --
naming one that is real but whose contents are not where the reader looks -- was not considered,
because "prefix" in the option's own description and "the directory itself" in its implementation were
never reconciled.

No test supplies a nested claim layout.

## References

- `BUG-56`, whose guard this defect bypasses.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
