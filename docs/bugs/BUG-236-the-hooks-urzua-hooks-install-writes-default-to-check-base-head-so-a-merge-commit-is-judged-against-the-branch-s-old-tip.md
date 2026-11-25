---
Stable-Id: 01M88RFEH77M98PZG8WZ7P8Z1M
Status: Open
Found-in: the investigation of `BUG-235`, reading where `urzua hooks install` takes the pre-commit arguments from
Realized-by:
---
# 236 — the hooks urzua hooks install writes default to check --base HEAD, so a merge commit is judged against the branch's old tip

## What was wrong

`hooks.pre-commit` defaults to `check --base HEAD` (`SPEC-3`, `SPEC-15`, `docs/reference/commands.md` near line 588, `ADR-71` near line 84). The hook `urzua hooks install` writes in another repository therefore has the defect `BUG-235` fixed in this repository's own hook: during a merge `HEAD` is the branch's old tip, so what came in from the branch merged in looks like changes with no attesting rows, and the commit is blocked by findings that `check --base <the base branch>` does not report.

## Why nothing caught it

No test of `hooks install` or `hooks run` runs the written hook during a merge, and the default was chosen for the ordinary commit, where `HEAD` is right.

## References

Open. The merge-base half is fixed by `BUG-235` in the engine: `--base MERGE_HEAD` now gives CI's view mid-merge. What remains is the adopters' default, `--base HEAD`. This is an engine change: the installed hook, or `hooks run`, has to choose `MERGE_HEAD` as the base when it exists, and `SPEC-3`, `SPEC-15`, `docs/reference/commands.md` and `ADR-71` state the default and need the same change (`ADR-71` as an amendment, since it is decided). A configured `hooks.pre-commit` that names its own base is the adopter's choice and stays as written. Found while fixing `BUG-235`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
