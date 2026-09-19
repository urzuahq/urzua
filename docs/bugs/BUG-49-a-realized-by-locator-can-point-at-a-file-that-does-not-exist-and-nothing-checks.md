---
Stable-Id: 01M2VNC7Y6Y36E4FD4Q0RB3T5A
Status: Fixed
Found-in: 'Asking why the engine did not catch ADR-4 reserving a `corpora/` directory that does not exist -- the answer generalised, and then found a live instance'
Regression-test: 'rust/crates/urzua-core/src/rules.rs::a_locator_naming_a_missing_path_is_an_error_observed_failing -- one present and one missing locator on the same record, asserting both are examined, one is reported, and an all-present record is silent'
---
# 49 — A `Realized-by` locator can point at a file that does not exist, and nothing checks

## What is wrong

Planting `Realized-by: code:rust/crates/urzua-core/src/DOES-NOT-EXIST.rs` on an ADR produces **zero
findings**. `check` is silent, `audit` is silent, `doctor` is silent.

A live instance exists. Audited across the corpus, 2026-09-19:

```text
129 locators declared, 1 pointing at nothing
  ADR-42 -> .urzua/config.toml
```

`ADR-52` deleted that file on 2026-09-17. `ADR-42`'s embodiment claim has been false for two days,
in a corpus this tool gates on every push.

## Why the engine cannot see it

The twenty rules check three relationships:

| | example |
|---|---|
| a record against **itself** | `header.required-fields`, `filename.title-consistency` |
| a record against **another record** | `pointer.resolution`, `relation.supersession-reciprocity` |
| a record against **git history** | `embodiment.consistency`'s drift detection (`ADR-32`) |

None checks a record against **the working tree**. `embodiment.consistency` comes closest and asks a
different question: whether a locator's *content* changed since the `Realized-by` line was last
touched. A locator that names nothing has no history to compare, so it drifts past the one rule built
to notice.

## Why it matters more than one stale field

The whole `Embodiment` model rests on locators. `Verified`, `Implemented` and `Specified` are computed
from `Realized-by` (`compute_embodiment`), so a record can claim its work is verified while naming a
file that was deleted -- and the claim reads as stronger than `Not started`, not weaker.

It is also the general shape behind the question that found it: `ADR-4` reserves a `corpora/`
directory that does not exist, and lists no `scripts/` directory that does. **The corpus can be
internally consistent and still describe a repository that is not there.** `SPEC-21` records the
layout half; this records the mechanism.

## Fix

A locator naming a path absent from the working tree is a finding. Opt-in and levelled like every rule
(`ADR-53`), because a repository mid-refactor may legitimately want it at `warn`.

Two constraints from things already learned:

- **`urzua-core` is pure** (`ADR-5`), so the CLI supplies existence, as it already supplies file
  contents to `claim.status-agreement`.
- **A path that has never existed and one recently deleted are the same finding**, and neither should
  be inferred from git. `BUG-40` is the report-side problem; this is one more rule that must not go
  silently out of scope.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-19 | Filed. **Why:** asked why the tool had not caught `ADR-4`'s stale directory table, given that it gates this repository on every push. It cannot: no rule compares a record to the working tree. Planting a nonexistent locator produced zero findings, and auditing all 129 found `ADR-42` claiming a file `ADR-52` deleted two days earlier. | **substantive** |
> | 2026-09-19 | `Status: Open` → `Fixed`. **Why:** `embodiment.locator-exists` reports a `Realized-by` locator absent from the working tree, opt-in and levelled like every rule (`ADR-53`), with the CLI supplying existence so `urzua-core` stays pure (`ADR-5`). Declared `error` here and it immediately caught the live instance -- `ADR-42` naming `.urzua/config.toml`, deleted by `ADR-52` two days earlier -- and blocked. That record is corrected. | **substantive** |
