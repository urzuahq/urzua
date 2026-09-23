---
Stable-Id: 01M36E3S45BC7849EPXGJEC940
Status: Fixed
Found-in: "A /code-review v0.3.0...main pass, round 21"
Regression-test: "a_staged_deletion_below_a_declared_dir_is_not_reported_as_outside_it_observed_failing, check_integration.rs"
---
# 126 — type.record-outside-declared-dir warns about a record staged for deletion

## What was wrong

`type_record_outside_declared_dir` is handed `discovered.paths` (git-tracked ∪ staged, per
`discover_tracked_files`'s own contract), and judges ownership purely by path structure. A record
staged for deletion (`git rm`, not yet committed) still appears in that set, and if it sits below a
declared type's `dir` without being directly in it (e.g. `docs/adr/archive/0001-x.md` under a declared
`docs/adr`), the rule reports it "sits below a declared record type's dir but not directly in it" --
a warning about a file the corpus is actively removing.

Reproduced: `git rm -q docs/adr/archive/0001-x.md` (staged, not committed), then `urzua check` still
reports the finding.

## Why nothing caught it

The rule's own comment cites `BUG-71` -- a staged deletion *directly inside* a declared dir was once
wrongly judged as sitting outside it, fixed by deciding ownership from `candidates`'s path structure
rather than the loaded record set. That fix didn't consider the case one level removed: a staged
deletion *not* directly inside any declared dir is still real, tracked-set data the rule examines, and
nothing excluded it from this rule's candidate set specifically.

## What changed

`check.rs` now filters `discovered.staged_deletions` out of the candidate list passed to
`type_record_outside_declared_dir`, the same way `load_records` already treats a staged deletion as
absent rather than examined.

## References

- `BUG-71`, `BUG-82` -- the precedent for treating a staged deletion as "going away," not as ordinary
  corpus content, in a structurally-decided rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-23 | Filed and fixed in one pass. **Why:** found by a full-release code review; a planted-violation test (`git rm` staged, not committed, on a record below-but-not-in a declared dir) observed failing before the fix. | **substantive** |
