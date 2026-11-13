---
Stable-Id: 01M2MF5SJQ1PRBKNHX4WFMVNDV
Status: Fixed
Found-in: 'building a synthetic corpus to reproduce BUG-23 -- a record whose only `# ` line sat inside a shell snippet reported a mismatch against that snippet'
Realized-by: code:rust/crates/urzua-core/src/rules/filename.rs#first_h1@sha256:41b9f17545b8441d83451f6d2bce19132ac431ad02baf3aa9e07371eb2cb2f64, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_ignores_an_h1_inside_a_fenced_block_observed_failing@sha256:393560522286aba7e81d3b857bf1b4ec14c4bab30b32d2f0d7d57b21a952e9bb, test:rust/crates/urzua-core/src/rules/filename.rs#filename_title_consistency_closes_a_fence_only_on_its_own_marker_observed_failing@sha256:28574d683b2a83d580129a16f978171259a9441a33ea5c22cf6a6d284ea92d6d
---
# 30 — filename.title-consistency matches an H1 inside a fenced code block and fabricates a mismatch

## What was wrong

`title_number` took the first line starting with `# ` anywhere in the document, with no tracking of
fenced regions. A `# ` inside a code block is sample text — a shell comment, a Markdown example, a
TOML comment — not a title.

Observed directly. A record containing no heading at all, only this:

```sh
# 1. install the thing
```

reported `filename claims number 8, but the H1 title claims 1`. That is not a wrong message about a
real defect; it is a **fabricated finding about a number that is not a record number**, and it names
a disagreement that does not exist.

The failure is conditional in a way that hides it: the fenced line is only reached when no real H1
precedes it. So the rule behaves correctly on every well-formed record and invents findings
specifically on the malformed ones — where a reader is least able to tell the difference.

For a foreign corpus this is worse than the defect `BUG-23` records. `BUG-23` produces a misleading
message about a real condition; this produces a confident, specific, entirely false claim.

## Why nothing caught it

Every record in this corpus carries a numbered H1 on the first heading line, so the search never
reaches the body, let alone a fence. Six files do contain fenced `# ` lines, but none of them is the
first such line in its file — the corpus cannot reach the branch. Nothing in the rule's tests
constructs a record without a heading.

The same shape as `BUG-29`, found in the same hour: a defect reachable only by a corpus this project
did not author.

## The fix

`first_h1` tracks ` ``` ` and `~~~` fences while scanning and skips lines inside them. It also skips
the frontmatter region, since a YAML comment line there starts with `# ` and would read as a title
for the same reason — but only when the header region opens at line 1, because blockquote and
bold-list headers sit *after* the H1 and skipping past those would skip the title itself.

## A second instance, caught in review

The first fix tracked fences with a boolean toggled by any line opening with three backticks or
tildes. That is not how a fence closes. A block opened with four backticks legitimately contains
three-backtick lines, and a backtick block contains `~~~` as ordinary text -- toggling on either
reopens the body mid-block, and the next sample heading becomes the title again. Demonstrated
against the same corpus: a record whose only heading sat inside a nested fence reported
`the H1 title '1. sample' claims 1`.

The fence now records its opening marker and length, and closes only on the same marker, at that
length or longer, with nothing following it.

Worth recording because the first fix was verified against a corpus that had no nested fences --
the same gap in coverage that let the original defect through, reproduced one level down.

## References

- `rust/crates/urzua-core/src/rules.rs` -- `first_h1`.
- BUG-23 -- the defect being fixed when this was found; this one is strictly worse against the same
  corpora.
- BUG-29 -- the other defect in the same function, same change.
- RFC-29 -- the deeper gap this does not close: the H1 numbering convention is assumed, not declared.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-16 | Filed and fixed in the same change. **Why:** a rule that invents a finding is a worse failure than one that words a real finding badly, and this one fires precisely where the reader has least context to doubt it. Found by reproduction, not by reading -- the branch is unreachable from this repo's own corpus. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
> | 2026-11-08 | Pinned (attested by beauwilliams, identity: gh-verified, actor: agent): 1 locators pinned, 0 file-level, 1 scoped. Evidence: rust/crates/urzua-core/src/rules/filename.rs#first_h1@sha256:41b9f17545b8 Reason: Pinned the locators re-pointed in this review of PR 433 against the code as it is now: record_id in rules/shared.rs still accepts any non-empty numeric prefix, values.rs normalize still strips leading zeros so ADR-0034 and ADR-34 are one id (BUG-2), filename_title_consistency in rules/filename.rs still separates an unnumbered H1 from an absent one and reports the H1's own line (BUG-23, BUG-29), first_h1 in rules/filename.rs still ignores an H1 inside a fenced block (BUG-30), discovery.rs scope_to_requested_paths still narrows check to the requested paths (BUG-1), and doctor.rs DoctorReport is still the JSON doctor emits (BUG-4). **Why:** a claim is renewed only over pinned code and test evidence. | **structural** |
> | 2026-11-08 | `Realized-by` re-pointed at where the evidence now lives: `code:` `rules.rs` replaced by `rules/filename.rs#first_h1`, checked against the code it names. **Why:** review of PR 433: the fix is intact but moved out of `rules.rs`. `first_h1` still tracks fenced code blocks, closing a fence only on its own marker at its own length or longer, and never reads a `# ` line inside one as the title; its two regression tests were already pinned and still resolve. | **substantive** |
