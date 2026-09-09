---
Stable-Id: 01M2465H1TT7VMFW7FWQVFJASW
Status: Planned
Phase: '0'
Track: schema-governance
---
# 92 — Three codebase-survey findings deferred from MILE-90

## What

Three real, low-risk findings surfaced by the same broader codebase survey that fed MILE-90, each
deliberately left unfixed there (MILE-90 scoped itself to the two cheapest/highest-value fixes:
rule-id `const` extraction and `main.rs::run_check`'s two-hand-synced-lists collapse, both shipped).
Filed for tracking, not fixed here — matching the `BUG-11`/`BUG-12` precedent from the same session.

- **`RealizedBy`'s three categories (`spec`/`code`/`test`) are hand-written as string literals
  independently in `rules.rs`'s parser and `graph.rs`'s `explain()`**, with nothing catching drift
  between the two lists if a category is ever added, renamed, or removed.
- **Severity (`Error`/`Warning`) is decided ad hoc per call site across all seventeen rule
  functions**, not owned by the rule itself as a declared default — `SPEC-2` already names this as
  the still-open MILE-80 question (severity from config vs. a richer taxonomy vs. a separate
  log-verbosity axis).
- **`new_record.rs`'s two rendering paths (`render_from_template`, `render_synthetic_yaml`) each
  independently hardcode `"Date"`/`"Author"`** as the two auto-filled fields, with nothing tying the
  two lists together if a third auto-filled field is ever added.

## Why

Found live during MILE-90's own broader-codebase-survey request ("look at the code base... come up
with other potential improvements to use modern, good Rust techniques"). Two cheapest/highest-value
findings from that same survey were folded directly into MILE-90 since it was already touching the
exact functions involved; these three were deliberately scoped out to keep MILE-90's own diff
focused on ADR-44's actual decision. Filing them here — rather than leaving them as a line in an
ephemeral session plan file — is the same discipline this project's own history keeps naming as a
recurring failure mode: a real finding noticed mid-session and then never actually tracked once the
immediate task closes.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-09-09 | Initial milestone, `Status: Planned`. Not yet scoped into concrete fix designs — each of the three findings needs its own design decision (e.g. whether `RealizedBy`'s categories become a shared `const` array or an enum) before implementation starts. | **structural** |
