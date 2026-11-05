---
Stable-Id: 01M2VH54E735WBS82D49WQMGSR
Status: Fixed
Found-in: 'A code review of the unreleased diff since v0.3.0, reproduced against a real changeset fixture'
Realized-by: test:rust/crates/urzua-core/src/rules/claim.rs#a_verb_inside_another_word_is_not_a_claim_observed_failing@sha256:426dacce13a21af6c5ef647d533280d26080aed3a15951828884ced6613704cc, test:rust/crates/urzua-core/src/rules/claim.rs#only_the_reference_the_verb_governs_is_claimed_observed_failing@sha256:5a7526016c6338130225c01c84006afb4a678c8f72db8cb5ba2244a2865c82ea
---
# 45 — `claim.status-agreement` matches a verb inside another word and claims every reference on the line

## What is wrong

Two independent defects in the same few lines, both producing **blocking false errors**.

**The verb test is a bare substring match.** `lower.contains("fixes")` matches `prefixes`;
`contains("closes")` matches `discloses`. Reproduced: the line

```
The path prefixes changed; see RFC-9 for why.
```

— which contains no closing verb at all — produced `error: claims to close RFC-9, but RFC-9 has
Status Draft`.

**Every reference on a verb-bearing line is treated as claimed.** The verb test is line-granular
while `scan_references` scans the whole line, so any co-mentioned record is claimed too. Reproduced:

```
Fixes BUG-39, which RFC-9 predicted.
```

`BUG-39` is `Fixed` and correctly silent. `RFC-9` is `Draft` and produced a blocking error, for a
record the line makes no claim about.

## Why it matters

`BUG-39` was about an extractor that under-reported. This is the same function over-reporting, and it
is worse: a false error blocks CI on a sentence that is correct English. The rule was written to catch
a false claim and is itself making them.

## Fix

Match the verb on word boundaries, and bind the claim to the reference the verb governs rather than to
the line. The second half is the harder one: *"Fixes BUG-39, which RFC-9 predicted"* needs the claim to
attach to the reference nearest the verb, not to all of them.

Narrow beats broad here. A missed claim is recoverable; a blocking false error on a correct sentence
teaches people to stop reading the output.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** review of the unreleased diff. The rule matches a verb as a substring, so ordinary words fire it, and it claims every reference on a matching line rather than the one the verb governs. Both produce blocking errors on correct prose. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** `claimed_closed` replaces the substring-and-whole-line match. Verbs are matched on word boundaries, so `prefixes` and `discloses` no longer fire, and a claim binds to the run of references immediately after the verb -- *"Fixes BUG-39, which RFC-9 predicted"* claims `BUG-39` only, while *"Closes BUG-39, BUG-40 and BUG-41"* claims all three. Narrow by choice: a missed claim is recoverable, a blocking false error on correct prose teaches people to stop reading the output. | **substantive** |
> | 2026-11-05 | `Regression-test` moved into `Realized-by` as 2 `test:` locators and the field retired (ADR-84). **Why:** ADR-84 makes `Realized-by` the one evidence field for every record type, so `status.delivery-agreement` can hold this record's `Status` to it. | **structural** |
