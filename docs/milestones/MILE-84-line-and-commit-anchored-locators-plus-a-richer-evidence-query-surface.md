---
Status: Planned
Stable-Id: 01M1Z8QEBSJRX510JWVK0155MP
Phase: '2'
Track: embodiment-model
Implements: —
Blocked-on: —
---
# 84 — Line and commit-anchored locators, plus a richer evidence-query surface

## What

Two related extensions to ADR-18's Embodiment model, decided together:

1. **Locator precision beyond file-level.** `Realized-by` locators are `spec:`/`code:`/`test:` plus
   a bare path today — no line number, no commit pin. Extend the syntax (a candidate shape:
   `code:path/to/file.rs@<short-sha>:<line>`) so a locator can survive an unrelated refactor of the
   same file while still pointing at the exact site, and so evidence can be cited in places a
   `// Implements: ADR-33`-style inline comment can't live at all — generated files, config data,
   infra-as-code, migrations.
2. **A richer evidence-query surface.** `urzua explain <path>` today only answers "which records cite
   this file." Extend it (or a differently-named command — naming is an open question, not decided
   here) to answer "show me every location — file, line, commit — that verifies or implements this
   record," surfacing the categorized locator breakdown (`spec`/`code`/`test`) directly rather than
   requiring a human to open each cited file and search.

This is also where ADR-18's own named-but-unbuilt follow-up lives: `embodiment.locator-promotion-
candidate` already flags a locator shared across multiple records as a promotion candidate for a
first-class `claim` record, but neither the `claim` type nor any query surface over it exists yet.
The richer evidence-query command may be exactly that missing query surface.

## Why

Raised live: a locator that's just a bare file path can't express "this specific line, as of this
specific commit" — which matters most exactly where an inline code comment can't be placed. Right
now, finding all the evidence for a record means grep-ing `Realized-by` across the corpus by hand
and manually opening each file; `explain` only does the first half.

## Done means

A `Realized-by` locator can name a line or a pinned commit (a shape such as `code:path/to/file.rs@<short-sha>:<line>`) and still resolve after an unrelated refactor of the same file; `check` reports a pinned line or commit that no longer exists; an evidence-query surface answers which records cite a given path, line or commit beyond `explain`'s file-level answer; the locator syntax is decided in an ADR, documented in `SPEC-2`, and used by at least one of urzua's own records.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|

> | 2026-11-04 | Done means added. **Why:** maintainer decision of 2026-11-04: every milestone, open or closed, states its Done means, and `section.required` enforces it. Derived from this record's What and Why; `Status` unchanged. | **structural** |
> | 2026-11-04 | The anchor slot is taken: `ADR-74` fixes the locator grammar `path[#scope][@anchor]`, so the candidate `path@<short-sha>:<line>` is written `path#L<line>@<short-sha>`, and a commit anchor is `@` and 7 to 64 lowercase hex. A locator carrying one parses and `embodiment.locator-exists` reports it "commit anchor not yet supported" until this milestone's ADR decides what a commit anchor verifies. `Status` unchanged. **Why:** `ADR-74` was accepted 2026-11-04 and binds the slot order so the two features cannot collide on `@`. | **substantive** |