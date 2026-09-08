---
Version: '0.2'
Date: 2026-08-20
Status: Draft
Implements: RFC-1, RFC-11
Parent: SPEC-1 (v0 CLI).
---
# SPEC-3 — Configuration: `.urzua/config.toml`

## Purpose

Config-driven is the founding claim, and this file is where it is falsifiable. The same binary must
serve two codebases with different layouts, record types and role requirements without either
forking it. If v0 needs a code change to work in the second codebase, the "build it once instead of
three times" claim is in trouble — and that is a finding about the schema, not a config bug.

That makes this spec's real job to be **specific enough to fail**. A config vague enough to express
anything proves nothing.

**Phasing.** SPEC-2 Phase A needs only record types and their directories — a real file, so the
config path exists from the first commit, but not this surface. Required fields, roles, boundaries
and rule severities land in Phase B; `doctor` with it. The falsifiable claim above is tested in
Phase C, against the second codebase, and cannot be tested before then.

## Resolution

`.urzua/config.toml`, written by `urzua init` (SPEC-5); `--config` overrides. The nearest
`.urzua/` at or above the cwd wins, so the tool works from a subdirectory of a monorepo without
arguments.

The config sits in a directory rather than at the root because it is not the only thing the tool
owns: templates live beside it at `.urzua/templates/`, and derived state at `.urzua/cache/`, which
is gitignored. Templates in particular must not live inside the corpus — a template is invalid *as
a record* by construction, so a checker discovers it and reports errors on a file that is correct.
SPEC-5 §Layout carries the full argument.

**One config per repo, not per package.** Per-package configs would let two packages disagree about
what a record *is*, and the cross-package questions — does this identifier resolve, do these scopes
overlap — become unanswerable when each side is validated under different rules. Per-package
variation is expressed as scoped overrides within the one file, which keeps the disagreement
visible in a single diff.

## What is configurable

```toml
[[record_type]]
name = "adr"
dir = "docs/adr"
filename = "{number}-{slug}.md"
statuses = ["Proposed", "Accepted", "Rejected", "Withdrawn", "Superseded", "Deprecated"]
header_shape = "blockquote"  # or "bold-list" / "yaml-frontmatter" -- RFC-10 §5, RFC-16: declared, never sniffed

[record_type.required_fields]
always = ["Status", "Date", "Author", "Deciders"]
# Keyed by status: the acceptance date exists only once there is an acceptance.
Accepted = ["Accepted"]

[record_type.roles]
author_may_self_review = false

[[boundary]]
paths = ["packages/sdk/**"]
kind = "published"
reason = "Published to the registry; identifiers here resolve to nothing for external readers."

[rules]
"header.duplicate-key" = "error"
"scope.matches-nothing" = "warn"
```

Configurable: record types and their directories, filename patterns, status enums, required fields
**per type and per status**, role requirements, back-pointer patterns, boundaries, and the severity
of every rule.

`init` materializes these from a built-in profile rather than referencing one by name, so every
rule in force is a visible line someone can disagree with (SPEC-5 §Type selection).

Two details carry weight:

- **Required fields are keyed by status, not only by type.** A field that only exists once a
  decision has been accepted is the common case, and a config that can only say "required" or "not"
  forces every corpus to choose between a false error and no check at all.
- **Every rule's severity is configurable, but no rule can be configured out of existence.** A rule
  set for a corpus is a claim about that corpus; silently dropping a rule and reporting `ok` is the
  failure this project exists to eliminate. Downgrade to `off` is permitted and is *reported* in
  `rulesExecuted` as skipped, never omitted.
- **The header shape is declared per record type, never auto-detected.** RFC-10 rejects sniffing a
  shape from content, for the same reason a rule can't be voted into existence by majority: a
  misconfigured shape must fail loud (`no header-shaped region found`) rather than silently matching
  the wrong lines. `header_shape` defaults to `"blockquote"` when omitted, so an existing config
  needs no change. `"yaml-frontmatter"` (RFC-16) is genuinely structured data, deserialized rather
  than line-recognized, and is what a from-scratch record generates by default; the other shapes stay
  what an adopted, pre-existing corpus is read as.

## Scope declaration

Per RFC-11 §6, a record's scope is a declared field on the record, not a config key — config
supplies only the **default** scope for a record type and the resolver's globbing rules.

The distinction matters: a default written into a new record by `urzua new` is a value an author can
see and correct; a default applied silently at read time is a fact nobody reviewed.

## `urzua doctor`

**Specified in full by SPEC-15.** Reports on the tool's own configuration and invocation health —
a real, standalone feature area with its own output shape and bug history, not a detail of how
config is structured.

## Success criteria

1. Both target codebases run green under one binary with no code change and no escape hatch.
2. Their two configs are diffable against each other, and the diff is a readable statement of how
   the corpora genuinely differ.
3. Every rule in SPEC-2 is reachable from config, and `doctor` (SPEC-15) lists the ones that are
   off — not yet true today, since no per-rule enable/disable or severity config exists yet
   (MILE-80).
4. If (1) fails, the failure is recorded as a schema finding with the specific rule that could not
   be expressed — not patched around with a code path for one repo.

Criterion 4 is the point of the spec. The claim is only worth something if we would notice it being
false.

## Open questions

- **Can scoped overrides express per-package variation without becoming a second config format?**
  The moment overrides gain their own resolution order, this is per-package configs with extra
  steps.
- **Does the config need a version field?** It has no consumers yet, so a version costs nothing now
  and is unaddable later without a migration.
- **Where do back-pointer patterns live** — config, or fixed by the record format (ADR-2)? Making
  them configurable is what lets a corpus keep its existing `Implements:` convention; fixing them is
  what makes cross-repo resolution possible (roadmap Phase 4).

## References

- SPEC-2 — the rules this configures.
- RFC-11 §1 and §6 — boundaries and scope.
- ADR-4 — the layout this config sits at the root of.

> **Revision log**
>
> | Date | Change | Class |
> |---|---|---|
> | 2026-08-20 | Config moves from `urzua.toml` at the repo root to `.urzua/config.toml`. The tool owns templates and derived state as well as config, and templates must sit outside the corpus they describe — a template is invalid as a record by construction, so a checker discovers it and reports errors on a correct file. The alternative, an ignore list, is the ad-hoc exclusion RFC-11 and RFC-13 both reject elsewhere. | **substantive** |
> | 2026-08-20 | Split out of SPEC-1's Configuration section and expanded. | **structural** |
> | 2026-09-07 | `urzua doctor` split out into its own spec (SPEC-15). **Why:** doctor is a real, standalone feature area with its own output shape and its own bug history (BUG-4, found the same day), not merely a detail of how config is structured — folding it into SPEC-3 by default rather than by a deliberate call was exactly the kind of inconsistency MILE-77's review named. Success criterion 3 corrected to note "which rules are off" isn't actually true yet (MILE-80). | **structural** |
