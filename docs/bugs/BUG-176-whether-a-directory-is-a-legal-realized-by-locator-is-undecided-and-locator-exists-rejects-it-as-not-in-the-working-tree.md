---
Stable-Id: 01M6D5VM1F4HBHKT51JKMX3HJZ
Status: Open
Found-in: "A /code-review pass, round 30"
Regression-test: "planned: a rules.rs test pinning the decided behavior for Realized-by: code:<existing directory>"
---
# 176 — whether a directory is a legal Realized-by locator is undecided and locator-exists rejects it as not in the working tree

## What was wrong

`Realized-by: code:src` where `src` is an existing directory is reported by `embodiment.locator-exists` as
"not in the working tree", and the message is also used for an untracked-but-on-disk file. Meanwhile git
pathspecs in drift detection accept a directory, so `embodiment.consistency` can compute drift for it.

## Why nothing caught it

`ADR-18` and `ADR-32` do not say whether a locator names a file or may name a tree. The check was written
for files.

## Why this needs a decision, not a mechanical fix

Needs a decision, not a mechanical fix: either a directory (and a glob) is a legal locator and the
existence check accepts a tracked prefix, or locators are files only and the message should say "not a
tracked file". Either way the wording "not in the working tree" asserts something the rule did not
establish for an existing directory or an untracked file.

## References

`BUG-175`.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-02 | Filed, left `Status: Open`. **Why:** found by a round 30 review and reproduced against a disposable fixture. | **substantive** |
