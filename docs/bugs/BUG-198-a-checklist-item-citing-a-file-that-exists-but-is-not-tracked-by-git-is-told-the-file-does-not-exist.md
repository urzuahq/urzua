---
Stable-Id: 01M7M8J9JY426M4JF5T89XJCHT
Status: Open
Found-in: "an adopting repository's report on 0.4.7, 2026-11-17"
Realized-by: null
---
# 198 — A checklist item citing a file that exists but is not tracked by git is told the file does not exist

## What was wrong

A record's checklist item cited two files the same change had just written, as `test:` and `code:`
locators, and the record was set to its done status before `git add`. `urzua check` reported, under
`status.delivery-agreement`, `locator tests/cli-hook.test.ts does not exist`. The file was on disk.
After `git add` the same check passed with nothing else changed.

Two things made the message misleading:

- The repository sets `discovery.include_untracked: true` and the run's summary reported
  `untracked_included: 2`, so the reader expected untracked files to count as evidence. That key
  governs which record files are discovered, not what an evidence locator resolves against, and
  nothing in the finding says so.
- The evidence header already tells the two cases apart: `embodiment.locator-exists` says a file
  "exists on disk but is not tracked by git" or "is not in the working tree"
  (`rust/crates/urzua-core/src/rules/embodiment.rs`). The checklist-item path does not:
  `missing_locator_cause` in `rust/crates/urzua-core/src/rules/delivery_items.rs` says "locator <path>
  does not exist: it was read as a path" for both. The rule already knows the difference:
  `with_locator_env` in the same file builds the set of files that are on disk and untracked, and
  `missing_locator_cause` never reads it. The same file is described two ways depending on where the
  record cites it.

Reproduction: a type declaring `statuses.delivery.items`; a record at the done status whose item
carries `evidence: test:new_file.rs`; create `new_file.rs` and do not `git add` it; run `urzua check`.

What a fix has to settle, for a decision before any build:

- Minimum: the item finding names the real state, "exists on disk but is not tracked by git", and its
  action is to `git add` the file, as the header finding already does.
- Open question: whether a locator may resolve against the working tree when
  `discovery.include_untracked` is on. A pin is a hash of committed content, so evidence that is not
  in git cannot be pinned; that argues for keeping the refusal and fixing only the message.

## Why nothing caught it

The header rule's tests cover a file that exists and is untracked. The checklist rule resolves
locators through its own path and its tests cover a missing file and a record identifier used as a
path, never a file that is present and untracked. Every fixture stages its evidence before checking.

## References

- `rust/crates/urzua-core/src/rules/delivery_items.rs`, `missing_locator_cause`: the message; `with_locator_env`: the untracked set it could use.
- `rust/crates/urzua-core/src/rules/embodiment.rs`: the header rule's two messages, the wording to match.
- `MILE-335`, `MILE-327`: the same family of findings, made clearer for other causes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-17 | Initial record, `Status: Open`. **Why:** an adopting repository on 0.4.7 cited two files it had just written in a checklist item, ran `urzua check` before `git add`, and was told the files do not exist. | **substantive** |
