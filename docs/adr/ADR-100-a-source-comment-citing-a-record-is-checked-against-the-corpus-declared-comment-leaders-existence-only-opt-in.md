---
Stable-Id: 01M7BEQXWQ185DXQ4Y2HM09E5X
Status: Proposed
Date: 2026-11-13
Author: beauwilliams
Deciders: beauwilliams
Embodiment: null
Supersedes / Superseded-by: null
Non-reference-ids: BUG-104
---
# 100 — A source comment citing a record is checked against the corpus: declared comment leaders, existence only, opt-in

## Context

`RFC-41` proposed `code.dangling-bug-reference`, the mirror of `embodiment.locator-exists`: a source comment that cites a record id (`BUG-104`, `ADR-57`) is a claim that the record exists, and nothing checked it. The incident was real: a record was deleted and renumbered, and two comments citing the deleted id outlived it until a human ran `grep`. The rule is built (`MILE-108`). `RFC-41` left four questions open, and `docs/rfc/README.md` makes building part of an RFC a decision that needs its own record. They are settled here.

Decided by the maintainer on 2026-11-13 (configurable comment leaders); the rest proposed by the orchestrator, pending maintainer review.

## Options considered

| Question | Option | Pros | Cons |
|---|---|---|---|
| Which comment syntaxes | A. A fixed list in the engine | No new key | The engine would know what a repository's languages are (`ADR-53`); a language with another leader is unreachable |
| | B. A declared list per extension, with the built-in set the back-pointer rules already read as the default (chosen) | Declared, not voted; an undeclared configuration reads exactly what it did; one reading of "a comment" for every source-reading rule | One more option, `code_comment_prefixes` |
| A citation in a string literal or a doc example | C. Parse each language to tell code from comment | Fewer false positives | The engine would carry a parser per language, against `ADR-53` |
| | D. Read any id after a leader; the declared `ignore_marker` excuses a line (chosen) | One rule for every language; the escape is already in the corpus (`ADR-80`) | A string such as `"x // ADR-9"` is read |
| Dangling versus superseded | E. Judge currency | Flags a comment citing a superseded record | A superseded record is still true history; every such comment would be flagged |
| | F. Judge existence only (chosen) | Matches the incident, which was deletion | A citation of a retired record passes |
| Scope of "source" | G. Also read non-record `.md` prose | Same staleness risk exists in prose | `citation.resolution` already reads record prose; other markdown has no declared unit, and a prose id is usually history, not a claim |
| | H. Only the files under declared paths with declared extensions, so `.md` only when declared (chosen) | Opt-in, no default scan | Prose outside the records is unchecked |

## Decision

In the context of source comments that cite records, facing a citation that silently outlives the record it names, the maintainer decided on 2026-11-13 that **comment leaders are declared per extension, falling back to the built-in leader set when none is declared**, and it is proposed, pending the maintainer's review, that **`code.dangling-bug-reference` also reads an id in a string literal or a doc-comment example after a leader like any other, with `ignore_marker` as the escape; skips an id inside a URL; leaves prose and `.md` files out of scope unless `extensions` declares them; and judges existence only**, to achieve a check that names no language and changes nothing for a repository that has not declared it, accepting that a string literal after a leader is a false positive until the line carries the marker.

1. **Leaders.** `code_comment_prefixes` is a map from an extension of the rule's `extensions` to a list of leaders (`rs: ["//"]`, `sql: ["--"]`). A leader that opens a line or follows whitespace starts a comment, and the longest leader at a position wins. An extension with no entry, and a rule with no option, read the leaders the back-pointer rules read. A key that is not one of the rule's `extensions`, an empty list, or a blank, whitespace-holding or repeated leader is a load error (`ADR-64`: structured config, no string mini-language, no silent no-op).
2. **String literals and doc examples.** Read. The test `a_string_literal_after_a_leader_and_a_doc_example_are_read_and_the_marker_excuses_them` pins it.
3. **URLs.** A word holding `://` is not read, so an issue link naming `BUG-9` is not a citation. An id glued to a URL in one word (`[ADR-60](https://x)`) is skipped with it: a missed citation, never a false positive.
4. **Out of scope.** Prose, `.md` files and any file outside the declared paths and extensions: a `.md` file is out of scope unless `extensions` declares `md`, in which case its lines are read with that extension's leaders.
5. **Existence only.** A citation of a superseded or retired record is live.
6. **Overlap.** A line `embodiment.back-pointer-resolution` already judges is left to that rule while it is enabled, so no line is reported twice.

## Reversibility

Easy. The rule and its options are opt-in; removing the option or the rule changes nothing for a repository that did not declare them. Narrowing the string-literal reading later is additive (a language-aware reader behind a declared key); widening to currency or prose is a new rule.

## Consequences

- A repository declares the leaders of its languages once, or takes the built-in set.
- `ignore_marker` lines are counted in the population's `excused_lines` and disclosed, and a marker on a line citing nothing is itself a finding, so the escape cannot rot silently.
- The built-in leader set stays the single reading of "a comment" for the back-pointer rules, so declaring leaders for this rule changes no back-pointer evidence.
- `MILE-108` stays open until this record is Accepted.

## References

- `RFC-41`: the proposal and its four open questions.
- `ADR-53`: declared, not voted.
- `ADR-64`: structured in, structured out.
- `ADR-80`: boundaries and the `ignore_marker`.
- `MILE-108`: the milestone that built the rule.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-11-13 | Drafted as Proposed. The configurable comment leaders are the maintainer's decision of 2026-11-13; the other settlements (string literals read with `ignore_marker`, prose and non-declared files out of scope, existence only, URL words skipped) are proposed by the orchestrator, pending the maintainer's review. **Why:** `RFC-41`'s open questions had to be settled in an ADR before `code.dangling-bug-reference` ships (`docs/rfc/README.md`). | **structural** |
