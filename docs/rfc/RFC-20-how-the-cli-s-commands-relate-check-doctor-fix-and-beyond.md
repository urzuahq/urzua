---
Stable-Id: 01M20SH8T1X2K6EVTB2TS4CGWT
Status: Draft
Date: 2026-09-08
Author: '@beauwilliams'
---
# 20 — How the CLI's commands relate: check, doctor, fix, and beyond

## Summary

Each command has its own spec, correctly scoped to itself, but no document says how they fit
together as one surface. Propose a two-axis taxonomy — *what it operates on* and *read-only vs.
writes* — as the reference for that relationship, kept separate from any single command's own spec.

## Motivation

Found live: answering "how is `doctor` different from `check`" required assembling the answer from
three separate specs (SPEC-2, SPEC-8, SPEC-15), none of which state the relationship — each
correctly documents itself, not its neighbors. That's the right call for each spec individually
(SPEC-15 already says explicitly: `check` validates records, whether `check` is correctly invoked is
a different question — but that line lives buried in `doctor`'s own Purpose, not somewhere a reader
would look first). Nothing catalogues the full set.

## Proposal

Two axes classify every command in the CLI as it exists today:

| Command | Operates on | Read-only / Writes |
|---|---|---|
| `check` (SPEC-2) | the corpus — every record against its declared schema | Read-only |
| `audit` (SPEC-11) | the corpus — cross-record only (supersession reciprocity, dangling refs), reusing `check`'s own rule functions (ADR-30) | Read-only |
| `doctor` (SPEC-15) | the tool's own setup — config, and if RFC-18 lands, environment preconditions | Read-only |
| `fix` (SPEC-8) | the corpus — one narrow, mechanically-derivable field (today) | Read-only (detect) / Writes (`--apply`) |
| `new` (SPEC-12) | the corpus — creates one record | Writes |
| `init` (SPEC-5) | the tool's own setup — proposes `.urzua/config.toml` from an existing corpus | Writes (`.urzua/` only, never the corpus) |
| `migrate` (SPEC-14) | the corpus — explicit, one-time, never automatic transformations (backfilling `Stable-Id`, previewing a new required field) | Writes (`ids`, opt-in) / Read-only (`schema --report`) |
| `explain`/`graph` (SPEC-13) | the corpus — read-only relationship queries over data every other command already parses | Read-only |

Two things this table makes visible that no single spec states on its own:

- **"Operates on the corpus" isn't one category.** `check`/`audit` validate; `fix` repairs a narrow
  slice; `new`/`migrate` create or transform; `explain`/`graph` only query. Conflating any two of
  these into one command was explicitly rejected each time it came up (`audit` vs. `check`'s ADR-30;
  `fix`'s narrow eligibility vs. a general repair tool, ADR-15).
- **`doctor` and `init` are the only two commands that don't touch the corpus at all** — one reports
  on the tool's setup, the other writes it. Everything else is corpus-shaped. This is exactly the
  axis RFC-18 proposes widening `doctor` along, not blurring.

## Open questions

- **Does RFC-18's widened `doctor` still cleanly stay off the corpus axis**, or does "does every
  type have a template" count as corpus-adjacent enough to blur this table's own claim? This RFC's
  taxonomy is only useful if the boundary holds; RFC-18 should be read against this table, not
  independently.
- **Is a `--type`/`--format dot` filter on `graph` (already named as deferred, not built, in SPEC-13)
  a new command or a flag?** This table treats it as within `graph`'s existing row; worth confirming
  that holds once/if it's built.
- **Should this table live somewhere more discoverable than an RFC once accepted** — a `--help`-level
  summary, or a section in SPEC-1 (the v0 CLI spec) that references this RFC's reasoning rather than
  restating it? Not deciding placement here, only the classification itself.

## Non-goals

- **Does not propose any new command** or change any existing command's scope — RFC-18/RFC-19 do
  that work; this RFC only names how the current (and RFC-18-widened) set relates.
- **Does not reopen ADR-30** (`audit` reusing `check`'s rule functions) or **ADR-15** (`fix`'s
  eligibility test) — both are already-decided boundaries this table catalogues, not reconsiders.

## References

- SPEC-1 — the v0 CLI spec every command here is a child of.
- SPEC-2/5/8/11/12/13/14/15 — each command's own complete build reference.
- ADR-15 — `fix`'s eligibility test, the reason it's narrowly scoped on the writes axis.
- ADR-30 — `audit` reusing `check`'s rule functions rather than being a third validator.
- RFC-18 — proposes widening `doctor`; this RFC's table is the frame that proposal should be read
  against.
- RFC-19 — proposes a hypothetical future `fix` tier; same relationship to this RFC as RFC-18's.
