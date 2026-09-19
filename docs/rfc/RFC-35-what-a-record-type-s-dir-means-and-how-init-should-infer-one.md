---
Stable-Id: 01M2W610G6NZXPRWHDZHG1HDPH
Status: Draft
Date: 2026-09-19
Author: beauwilliams
Supersedes / Superseded-by: —
---
# 35 — What a record type's `dir` means, and how `init` should infer one

## Summary

A record type's `dir` is matched by **path prefix** (`discovery.rs:68`), so a type claims an entire
subtree. Subtrees nest, so types overlap, and nothing in the schema says which type wins. Four
heuristics have now been written to guess what an adopter meant, each patching the previous one's edge
case.

Propose deciding the question underneath them: **`dir` means *this directory*, and recursion is
declared.**

## The four heuristics

| record | heuristic added |
|---|---|
| `BUG-36` | derive `dir` from each record's own parent directory |
| `BUG-43` | when two proposed dirs nest, the **outer** wins and the inner folds into it |
| `BUG-54` | ...unless the outer holds more than one type beneath it |
| review of `#76` | ...and then the outer is dropped entirely, because a container type double-counts |

Each is correct against the case that produced it. None was declared by anybody. Together they are an
inference engine guessing at corpus intent, and the guess is load-bearing: `check` reported
`files_examined: 8` for four records before the last one landed.

## Where this actually bites

**Not in this repository.** Its six types each live in their own subdirectory, no record sits loose in
`docs/`, and its config was hand-written and converted rather than generated. Every instance was found
by a synthetic fixture.

It bites an **adopter running `init`** on a corpus that is not already tidy -- which is the entire
population `init` exists for, and which `MILE-51` keeps demonstrating this project is bad at. Our own
corpus is too clean to expose it, which is `MILE-101`'s argument for an acceptance suite in one
sentence.

## The two halves, which are separable

**1. What `dir` means at `check` time.** Prefix matching is latent, not broken: it only produces
overlap when a container type is declared, and no hand-written config declares one. But it is
undeclared behaviour, and "this directory" versus "this subtree" is exactly the kind of thing
`ADR-53` says a repository should state rather than inherit.

```yaml
adr:
  dir: docs/adr             # this directory
rfc:
  dir: docs/rfc
  recursive: true           # ...and everything beneath it
```

Explicit-by-default with opt-in recursion is what `ESLint`, `.gitignore` and `ripgrep` all settle on.
It makes `BUG-43`'s `archive/` case a **declaration** rather than an inference, and two types can then
only overlap if someone declared them overlapping -- which is their statement, not ours.

**2. How `init` infers one.** Even with `dir` decided, adopt mode still has to propose something from
a directory tree it has never seen. The heuristics do not disappear; they become *proposals* an adopter
reads and edits, which is a much weaker thing to get wrong than a matching rule.

Worth separating because the first is a schema decision and the second is a UX one, and conflating
them is how four heuristics ended up inside a matching rule.

## Why this is `Draft` and not a decision

Two things are unresearched:

- **Whether any real corpus wants recursion.** `npryce/adr-tools` does not. MADR does not. `archive/`
  and `superseded/` subdirectories are asserted to be common and have not been counted.
- **What `init` should do with a stray record-shaped file in a parent directory.** Propose nothing and
  say so, propose a type and let the adopter delete it, or refuse to adopt until the corpus is
  unambiguous. The third is the most honest and the least usable.

## Relationship to `MILE-98`

`MILE-98` declares how to find a record's **identity, fields and sections**. *Which records belong to a
type* is the same class of question and is currently answered by an undeclared prefix rule -- so this
either belongs in that milestone's scope or immediately beside it. Deciding it first would delete four
heuristics rather than carry them into the new model.

## References

- ADR-53 -- governance is declared, not inherited; this is one more thing currently inherited.
- BUG-36, BUG-43, BUG-54 -- the heuristics, in the order they accreted.
- MILE-51 -- the adopter population this affects and this repository does not represent.
- MILE-98 -- the declared document model this sits beside.
- MILE-101 -- why a corpus this clean cannot find these.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed, `Status: Draft`. **Why:** four heuristics have accreted inside `init` and `check` to guess which records belong to a type, each patching the last one's edge case, and the question underneath them -- whether `dir` means a directory or a subtree -- has never been decided. Raised while asking why `init` proposed `docs/` as a type when the config named `docs/adr`. | **substantive** |
