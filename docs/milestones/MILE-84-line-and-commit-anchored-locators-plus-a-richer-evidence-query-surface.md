# 84 — Line and commit-anchored locators, plus a richer evidence-query surface

> Status: Planned
> Stable-Id: 01M1Z8QEBSJRX510JWVK0155MP
> Phase: 2
> Track: embodiment-model
> Implements: —

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

## Blocked on

`—`

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
